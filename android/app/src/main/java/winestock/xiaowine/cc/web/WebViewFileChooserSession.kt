package winestock.xiaowine.cc.web

/**
 * 管理 WebView 文件选择（`onShowFileChooser`）的单 pending 回调所有权。
 *
 * 本类只拥有 ValueCallback 生命周期与 Activity Result → URI 列表映射，不依赖 WebView、
 * 不启动 Intent、不申请存储权限，因此可通过纯 JVM 单元测试验证单次结算与竞态。
 * Intent 启动与 `content://` 读取留在 Activity；本类从不把 URI 转成“真实文件路径”。
 *
 * 与 MainActivity 中**单个** [androidx.activity.result.ActivityResultLauncher] 配套：
 * supersede 后再次 launch 时，AndroidX 对同一 launcher 只回传一次结果（对应当前选择器），
 * 因此 [deliver] 始终结算**当前** pending，不得用“丢弃 N 次 stale 结果”的计数模型。
 */
internal class WebViewFileChooserSession {
    /**
     * 对应 [android.webkit.ValueCallback.onReceiveValue]：`uris == null` 表示取消或失败，
     * 非空数组为用户选定的 content URI 字符串（保持 `content://` 等形式，不做路径反查）。
     */
    fun interface Callback {
        fun onReceiveValue(uris: Array<String>?)
    }

    private var pending: Callback? = null
    private var destroyed = false

    /**
     * 登记新的文件选择回调。
     *
     * 若已有未结算回调，先以 `null` 结算旧回调一次，避免 WebView 永久挂起。
     * 随后 Activity 会再次 launch 同一 launcher；唯一的 Activity Result 交给新 pending。
     * 已 [destroy] 时立即以 `null` 结算新回调并返回 false，调用方不得再 launch 选择器。
     */
    fun begin(callback: Callback): Boolean {
        if (destroyed) {
            callback.onReceiveValue(null)
            return false
        }
        settlePending(null)
        pending = callback
        return true
    }

    /**
     * 以选定 URI 结算当前回调。空数组与 null 均按取消处理（向 WebView 传 null）。
     *
     * 与单 launcher 约定一致：本次结果属于当前 pending（含 supersede 后重新 launch 的那次）。
     * @return true 表示确实结算了当前 pending；无 pending 或已 destroy 返回 false。
     */
    fun deliver(uris: Array<String>?): Boolean {
        if (destroyed) return false
        val current = pending ?: return false
        pending = null
        current.onReceiveValue(normalize(uris))
        return true
    }

    /** 用户取消、选择器启动失败或无可用 Activity 时，以 null 结算当前回调。 */
    fun cancel(): Boolean = deliver(null)

    /**
     * 销毁会话：以 null 结算 pending，之后拒绝新 begin（立即 null 结算）与 deliver。
     * Activity onDestroy 必须调用，防止泄漏回调。
     */
    fun destroy() {
        if (destroyed) return
        destroyed = true
        settlePending(null)
    }

    /** 是否仍有未结算的文件选择回调（测试与诊断用）。 */
    fun hasPending(): Boolean = pending != null

    private fun settlePending(uris: Array<String>?) {
        val current = pending ?: return
        pending = null
        current.onReceiveValue(normalize(uris))
    }

    companion object {
        /**
         * 将系统文件选择器的 Activity Result 映射为 URI 字符串数组。
         *
         * - `resultOk == false`（用户取消 / RESULT_CANCELED 等）→ null
         * - 多选 ClipData 优先；否则使用单 data URI
         * - 成功但无任何 URI → null（与取消一致，结束 WebView 回调）
         *
         * Activity 负责从 Intent 拆出 `dataUri` 与 `clipUris`；本函数保持纯解析、无 Android 类型依赖。
         */
        fun mapChooserResult(
            resultOk: Boolean,
            dataUri: String?,
            clipUris: List<String>,
        ): Array<String>? {
            if (!resultOk) return null
            val fromClip =
                clipUris
                    .asSequence()
                    .map { it.trim() }
                    .filter { it.isNotEmpty() }
                    .toList()
            if (fromClip.isNotEmpty()) {
                return fromClip.toTypedArray()
            }
            val single = dataUri?.trim().orEmpty()
            if (single.isNotEmpty()) {
                return arrayOf(single)
            }
            return null
        }

        private fun normalize(uris: Array<String>?): Array<String>? {
            if (uris == null || uris.isEmpty()) return null
            val cleaned =
                uris
                    .asSequence()
                    .map { it.trim() }
                    .filter { it.isNotEmpty() }
                    .toList()
            return if (cleaned.isEmpty()) null else cleaned.toTypedArray()
        }
    }
}
