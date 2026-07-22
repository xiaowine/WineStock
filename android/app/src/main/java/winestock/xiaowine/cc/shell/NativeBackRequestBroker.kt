package winestock.xiaowine.cc.shell

/**
 * 管理 Android 原生返回与当前 WebView 页面之间的一次性协商。
 *
 * 本类只拥有 requestId、页面代次、单 pending 状态、超时与幂等结算；它不依赖 Android UI、
 * 不发送 Bridge 信封，也不执行 WebView 或 Activity fallback，因此可以通过纯 JVM 单元测试验证竞态。
 */
internal class NativeBackRequestBroker(
    private val responseTimeoutMs: Long,
    private val scheduler: TimeoutScheduler,
) {
    /** Native 发给前端的返回请求。 */
    data class Request(
        val requestId: String,
        val pageGeneration: Long,
        val canGoBack: Boolean,
    )

    /** 发起请求的结果；已有 pending 时重复返回会被消费而不排队。 */
    sealed interface BeginResult {
        data class Started(val request: Request) : BeginResult

        data object AlreadyPending : BeginResult

        data object Destroyed : BeginResult
    }

    /** 可取消的超时任务句柄。 */
    fun interface TimeoutHandle {
        fun cancel()
    }

    /** 注入式超时调度器，Android 使用 Handler，测试使用确定性假时钟。 */
    fun interface TimeoutScheduler {
        fun schedule(delayMs: Long, action: () -> Unit): TimeoutHandle
    }

    private data class PendingRequest(
        val request: Request,
        val onSettled: (handled: Boolean) -> Unit,
        var timeoutHandle: TimeoutHandle? = null,
    )

    private var pageGeneration = 0L
    private var requestSequence = 0L
    private var pending: PendingRequest? = null
    private var destroyed = false

    /** 当前页面开始加载时进入新代次，并让旧页面尚未完成的请求静默失效。 */
    fun beginPage(): Long {
        if (destroyed) return pageGeneration
        cancelPending()
        pageGeneration += 1
        return pageGeneration
    }

    /**
     * 建立一次协商并安排超时。调用方负责把 Started.request 发送给前端；发送失败时应调用
     * [cancelRequest]，再由 Activity 立即执行 fallback。
     */
    fun beginRequest(
        canGoBack: Boolean,
        onSettled: (handled: Boolean) -> Unit,
    ): BeginResult {
        if (destroyed) return BeginResult.Destroyed
        if (pending != null) return BeginResult.AlreadyPending

        requestSequence += 1
        val request =
            Request(
                requestId = "page-$pageGeneration:$requestSequence",
                pageGeneration = pageGeneration,
                canGoBack = canGoBack,
            )
        val pendingRequest = PendingRequest(request = request, onSettled = onSettled)
        pending = pendingRequest
        val timeoutHandle =
            scheduler.schedule(responseTimeoutMs) {
                settleIfCurrent(request.requestId, handled = false)
            }
        if (pending === pendingRequest) {
            pendingRequest.timeoutHandle = timeoutHandle
        } else {
            timeoutHandle.cancel()
        }
        return BeginResult.Started(request)
    }

    /** 匹配当前 requestId 并结算一次；迟到、重复、未知或旧页面应答返回 false。 */
    fun resolve(requestId: String, handled: Boolean): Boolean =
        settleIfCurrent(requestId, handled)

    /** 发送 Bridge 事件失败时，只取消对应请求，不触发结算回调。 */
    fun cancelRequest(requestId: String): Boolean {
        val current = pending ?: return false
        if (current.request.requestId != requestId) return false
        pending = null
        current.timeoutHandle?.cancel()
        return true
    }

    /** 页面导航、Activity pause 等生命周期变化会静默取消当前请求，不额外 fallback。 */
    fun cancelPending() {
        val current = pending ?: return
        pending = null
        current.timeoutHandle?.cancel()
    }

    /** 销毁 broker；之后不再接受新请求或执行旧回调。 */
    fun destroy() {
        if (destroyed) return
        destroyed = true
        cancelPending()
    }

    private fun settleIfCurrent(requestId: String, handled: Boolean): Boolean {
        val current = pending ?: return false
        if (current.request.requestId != requestId) return false
        pending = null
        current.timeoutHandle?.cancel()
        current.onSettled(handled)
        return true
    }
}
