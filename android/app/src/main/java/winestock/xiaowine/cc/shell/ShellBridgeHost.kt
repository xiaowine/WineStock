package winestock.xiaowine.cc.shell

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.webkit.JavaScriptReplyProxy
import androidx.webkit.WebMessageCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONObject
import java.io.IOException

/**
 * Android shell 的 Shell Bridge v1 原生分发。
 *
 * 职责：在受信任 origin 上注册 WebMessageListener 通道、在文档起始注入 assets/shell/bridge.js，
 * 解析前端请求信封并路由到运行配置、具名平台能力与本地服务生命周期处理，通过 JavaScriptReplyProxy
 * 回复，并向前端推送运行状态、应用恢复和原生返回事件。
 *
 * 边界：只处理运行配置、服务生命周期和具名平台事件，不代理业务 HTTP、不传递 token、
 * 不暴露通用 native 调用。
 * 本地 Axum 端上服务尚未实现，本类对本地模式返回稳定的 unsupported_runtime_mode。
 *
 * 传输协议与 assets/shell/bridge.js 对齐：
 *   - JS -> Native 请求：{ type:"call", id, method, params }
 *   - Native -> JS 回复：{ type:"reply", id, ok, result?, error? }
 *   - Native -> JS 事件：{ type:"event", event, payload? }
 */
class ShellBridgeHost(
    context: Context,
    private val deviceName: String,
    private val appVersion: String,
    private val nativeBackResponseTimeoutMs: Long,
    /** 前端报告首屏就绪时回调，用于隐藏加载遮罩；在主线程调用。 */
    private val onFrontendReady: () -> Unit = {},
) {
    private val appContext = context.applicationContext
    private val store = RuntimeConfigStore(appContext)
    private val mainHandler = Handler(Looper.getMainLooper())
    private val nativeBackBroker =
        NativeBackRequestBroker(
            responseTimeoutMs = nativeBackResponseTimeoutMs,
            scheduler =
                NativeBackRequestBroker.TimeoutScheduler { delayMs, action ->
                    val runnable = Runnable(action)
                    mainHandler.postDelayed(runnable, delayMs)
                    NativeBackRequestBroker.TimeoutHandle {
                        mainHandler.removeCallbacks(runnable)
                    }
                },
        )

    /** 保存最近的回复代理，用于向当前页面推送事件。页面导航或重建时会被替换。 */
    private var replyProxy: JavaScriptReplyProxy? = null

    /** 当前页面完成 frontendReady 后确认的代理；native back 事件只通过该代理发送。 */
    private var readyReplyProxy: JavaScriptReplyProxy? = null

    private var installed = false
    private var currentPageTrusted = false
    private var currentPageGeneration = 0L
    private var frontendReady = false
    private var activityResumed = false
    private var destroyed = false

    /** 平台能否安装消息通道与文档起始脚本；任一能力缺失则桥不可用。 */
    val isSupported: Boolean =
        WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER) &&
            WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)

    /** Activity 对一次返回提交的分发结果。 */
    enum class NativeBackDispatchResult {
        DISPATCHED,
        ALREADY_PENDING,
        UNAVAILABLE,
    }

    /**
     * 在加载前端前安装桥。必须在 WebView.loadUrl 之前调用，保证 document-start 脚本
     * 在页面脚本执行前注入。返回是否成功安装。
     */
    fun install(webView: WebView): Boolean {
        if (!isSupported) {
            return false
        }
        WebViewCompat.addWebMessageListener(
            webView,
            CHANNEL_NAME,
            setOf(TRUSTED_ORIGIN),
        ) { _, message, sourceOrigin, isMainFrame, proxy ->
            // 再次确认来源：只接受受信任 origin 主框架的消息。
            if (
                destroyed ||
                !currentPageTrusted ||
                !isMainFrame ||
                !isTrustedOrigin(sourceOrigin)
            ) {
                return@addWebMessageListener
            }
            replyProxy = proxy
            handleMessage(message, proxy)
        }
        // 文档起始注入：先定义元数据，再执行 shim。origin 限制与消息通道一致。
        WebViewCompat.addDocumentStartJavaScript(
            webView,
            buildBootstrapScript(),
            setOf(TRUSTED_ORIGIN),
        )
        installed = true
        return true
    }

    /** 主框架开始加载时进入新页面代次，并取消旧页面尚未结算的 native back 请求。 */
    fun onPageStarted(url: String?) {
        if (destroyed) return
        currentPageGeneration = nativeBackBroker.beginPage()
        currentPageTrusted = isTrustedPageUrl(url)
        frontendReady = false
        replyProxy = null
        readyReplyProxy = null
    }

    /** Activity 恢复可交互状态；页面仍需 frontendReady 后才能接收 native back。 */
    fun onActivityResumed() {
        if (!destroyed) activityResumed = true
    }

    /** Activity 离开前台时静默取消 pending，防止后台 timeout 再触发 fallback。 */
    fun onActivityPaused() {
        activityResumed = false
        nativeBackBroker.cancelPending()
    }

    /** 销毁 Bridge 生命周期；之后不再发事件、执行结算回调或接受新请求。 */
    fun destroy() {
        if (destroyed) return
        destroyed = true
        installed = false
        activityResumed = false
        frontendReady = false
        currentPageTrusted = false
        replyProxy = null
        readyReplyProxy = null
        nativeBackBroker.destroy()
    }

    /**
     * 向已 ready 的当前可信页面发起一次原生返回协商。
     * handled=false 或超时只回调 Activity 一次；等待期间的重复返回由 broker 直接消费。
     */
    fun requestNativeBack(
        canGoBack: Boolean,
        onResolved: (handled: Boolean) -> Unit,
    ): NativeBackDispatchResult {
        val proxy = readyReplyProxy
        if (
            destroyed ||
            !installed ||
            !activityResumed ||
            !currentPageTrusted ||
            !frontendReady ||
            proxy == null
        ) {
            return NativeBackDispatchResult.UNAVAILABLE
        }

        val requestGeneration = currentPageGeneration
        val beginResult =
            nativeBackBroker.beginRequest(canGoBack) { handled ->
                mainHandler.post {
                    if (
                        !destroyed &&
                        installed &&
                        activityResumed &&
                        currentPageTrusted &&
                        frontendReady &&
                        requestGeneration == currentPageGeneration
                    ) {
                        onResolved(handled)
                    }
                }
            }
        return when (beginResult) {
            NativeBackRequestBroker.BeginResult.AlreadyPending ->
                NativeBackDispatchResult.ALREADY_PENDING
            NativeBackRequestBroker.BeginResult.Destroyed -> NativeBackDispatchResult.UNAVAILABLE
            is NativeBackRequestBroker.BeginResult.Started -> {
                val request = beginResult.request
                val envelope =
                    JSONObject()
                        .put("type", "event")
                        .put("event", "nativeBackRequested")
                        .put(
                            "payload",
                            JSONObject()
                                .put("requestId", request.requestId)
                                .put("canGoBack", request.canGoBack),
                        )
                try {
                    postEnvelope(proxy, envelope)
                    NativeBackDispatchResult.DISPATCHED
                } catch (_: Exception) {
                    nativeBackBroker.cancelRequest(request.requestId)
                    NativeBackDispatchResult.UNAVAILABLE
                }
            }
        }
    }

    /** 通知前端应用从后台恢复，触发服务可用性补检。 */
    fun notifyAppResumed() {
        val proxy = readyReplyProxy ?: return
        val envelope =
            JSONObject()
                .put("type", "event")
                .put("event", "appResumed")
        postEnvelope(proxy, envelope)
    }

    private fun handleMessage(message: WebMessageCompat, proxy: JavaScriptReplyProxy) {
        val data = message.data ?: return
        val envelope =
            try {
                JSONObject(data)
            } catch (_: Exception) {
                return
            }
        if (envelope.optString("type") != "call") {
            return
        }
        // id 缺失说明前端未遵守协议，无法回复，直接忽略。
        if (envelope.isNull("id")) {
            return
        }
        val id = envelope.get("id")
        val method = envelope.optString("method")
        val params = envelope.optJSONObject("params")

        try {
            val result = dispatch(method, params, proxy)
            replySuccess(proxy, id, result)
        } catch (error: BridgeException) {
            replyError(proxy, id, error.code, error.message ?: "Shell Bridge 调用失败")
        } catch (error: Exception) {
            replyError(
                proxy,
                id,
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                error.message ?: "Shell Bridge 调用失败",
            )
        }
    }

    /** 路由方法调用，返回值将作为 reply.result 序列化。 */
    private fun dispatch(
        method: String,
        params: JSONObject?,
        proxy: JavaScriptReplyProxy,
    ): Any? =
        when (method) {
            "getRuntimeSnapshot" -> loadInitialSnapshot()
            "validateRuntimeConfig" -> validateConfig(requireConfig(params))
            "applyRuntimeConfig" -> applyConfig(requireConfig(params))
            // 端上本地服务尚未实现，生命周期操作返回带错误的当前快照，而非抛出。
            "startLocalService",
            "stopLocalService",
            "restartLocalService" -> unsupportedLocalService()
            "frontendReady" -> {
                frontendReady = true
                readyReplyProxy = proxy
                onFrontendReady()
                null
            }
            "resolveNativeBack" -> resolveNativeBack(params)
            "openExternal" -> {
                openExternal(requireString(params, "url"))
                null
            }
            else ->
                throw BridgeException(
                    ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                    "未知的 Shell Bridge 方法：$method",
                )
        }

    /** 依据持久化配置派生初始快照，对齐 web.ts 的 loadInitialSnapshot 三态。 */
    private fun loadInitialSnapshot(): JSONObject =
        when (val loaded = store.load()) {
            is RuntimeConfigStore.Loaded.Present -> {
                val validation = RuntimeConfigValidator.validate(loaded.config)
                if (validation.valid) {
                    RuntimeSnapshotFactory.configured(
                        loaded.config,
                        nativeBackSupported = installed,
                    )
                } else {
                    RuntimeSnapshotFactory.invalid(
                        loaded.config,
                        "已保存的运行配置无效，请修正后重新应用",
                        nativeBackSupported = installed,
                    )
                }
            }
            is RuntimeConfigStore.Loaded.Invalid ->
                RuntimeSnapshotFactory.invalid(
                    loaded.fallback,
                    "已保存的运行配置无法解析，请重新应用默认配置",
                    nativeBackSupported = installed,
                )
            RuntimeConfigStore.Loaded.Missing ->
                RuntimeSnapshotFactory.unconfigured(nativeBackSupported = installed)
        }

    private fun validateConfig(config: EditableRuntimeConfig): JSONObject {
        val result = RuntimeConfigValidator.validate(config)
        return validationResultJson(result)
    }

    /**
     * 应用配置。远端模式格式合法即持久化（可达性由前端健康检查负责）；
     * 本地模式暂不支持，返回校验通过但 applied=false 的 unsupported 结果，且不持久化。
     */
    private fun applyConfig(config: EditableRuntimeConfig): JSONObject {
        val validation = RuntimeConfigValidator.validate(config)
        if (!validation.valid) {
            return applyResultJson(
                validation = validation,
                applied = false,
                snapshot = loadInitialSnapshot(),
                error = null,
            )
        }

        if (RuntimeModes.isLocal(config.mode)) {
            return applyResultJson(
                validation = validation,
                applied = false,
                snapshot = RuntimeSnapshotFactory.withError(
                    loadInitialSnapshot(),
                    ShellErrorCodes.UNSUPPORTED_RUNTIME_MODE,
                    "当前 Android 版本暂不支持在设备上运行本地 WineStock 服务，请连接远程服务",
                ),
                error =
                    ShellError(
                        ShellErrorCodes.UNSUPPORTED_RUNTIME_MODE,
                        "当前 Android 版本暂不支持在设备上运行本地 WineStock 服务，请连接远程服务",
                        RuntimeConfigFields.MODE,
                    ),
            )
        }

        if (!store.save(config)) {
            return applyResultJson(
                validation = validation,
                applied = false,
                snapshot = loadInitialSnapshot(),
                error =
                    ShellError(
                        ShellErrorCodes.CONFIG_UNAVAILABLE,
                        "无法保存运行配置，请检查设备存储权限",
                        null,
                    ),
            )
        }

        val snapshot = RuntimeSnapshotFactory.configured(config, nativeBackSupported = installed)
        publishSnapshot(snapshot)
        return applyResultJson(
            validation = validation,
            applied = true,
            snapshot = snapshot,
            error = null,
        )
    }

    private fun unsupportedLocalService(): JSONObject =
        RuntimeSnapshotFactory.withError(
            loadInitialSnapshot(),
            ShellErrorCodes.UNSUPPORTED_RUNTIME_MODE,
            "当前 Android 版本不能直接管理本地 WineStock 服务",
        )

    /** 结算当前 native back 请求；未知、重复、迟到或旧页面 requestId 稳定返回 accepted=false。 */
    private fun resolveNativeBack(params: JSONObject?): JSONObject {
        val requestId = requireNativeBackRequestId(params)
        val handledValue = params?.opt("handled")
        if (handledValue !is Boolean) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "原生返回应答缺少 handled 布尔值",
            )
        }
        val reason = params.opt("reason")
        if (reason !is String || reason !in NATIVE_BACK_REASONS) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "原生返回应答 reason 无效",
            )
        }
        return JSONObject().put(
            "accepted",
            nativeBackBroker.resolve(requestId = requestId, handled = handledValue),
        )
    }

    /** 通过系统浏览器打开经过校验的外部链接；只允许不含凭据的 http/https。 */
    private fun openExternal(url: String) {
        val normalized = RuntimeConfigValidator.normalizeApiBaseUrl(url)
        // normalizeApiBaseUrl 会剥离路径，openExternal 需要保留完整地址，因此单独校验后原样打开。
        val target =
            try {
                Uri.parse(url.trim())
            } catch (_: Exception) {
                throw BridgeException(ShellErrorCodes.INVALID_BRIDGE_PAYLOAD, "外部链接无效")
            }
        val scheme = target.scheme?.lowercase()
        if ((scheme != "http" && scheme != "https") || target.userInfo != null) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "外部链接必须使用不含凭据的 http 或 https 地址",
            )
        }
        // normalized 为 null 时说明 host 非法（如 0.0.0.0），一并拒绝。
        if (normalized == null && target.host.isNullOrBlank()) {
            throw BridgeException(ShellErrorCodes.INVALID_BRIDGE_PAYLOAD, "外部链接无效")
        }
        val intent = android.content.Intent(android.content.Intent.ACTION_VIEW, target)
        intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            appContext.startActivity(intent)
        } catch (error: Exception) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "无法打开外部链接：${error.message}",
            )
        }
    }

    private fun publishSnapshot(snapshot: JSONObject) {
        val proxy = readyReplyProxy ?: replyProxy ?: return
        val envelope =
            JSONObject()
                .put("type", "event")
                .put("event", "runtimeStateChanged")
                .put("payload", snapshot)
        postEnvelope(proxy, envelope)
    }

    private fun replySuccess(proxy: JavaScriptReplyProxy, id: Any, result: Any?) {
        val envelope =
            JSONObject()
                .put("type", "reply")
                .put("id", id)
                .put("ok", true)
        if (result != null) {
            envelope.put("result", result)
        }
        postEnvelope(proxy, envelope)
    }

    private fun replyError(proxy: JavaScriptReplyProxy, id: Any, code: String, message: String) {
        val envelope =
            JSONObject()
                .put("type", "reply")
                .put("id", id)
                .put("ok", false)
                .put(
                    "error",
                    JSONObject().put("code", code).put("message", message),
                )
        postEnvelope(proxy, envelope)
    }

    private fun postEnvelope(proxy: JavaScriptReplyProxy, envelope: JSONObject) {
        proxy.postMessage(envelope.toString())
    }

    private fun buildBootstrapScript(): String {
        val meta =
            JSONObject()
                .put("channelName", CHANNEL_NAME)
                .put("deviceName", deviceName)
                .put("appVersion", appVersion)
        val shim = readAssetText(SHIM_ASSET_PATH)
        return "window.__WINESTOCK_BRIDGE_META__ = $meta;\n$shim"
    }

    private fun readAssetText(path: String): String =
        try {
            appContext.assets.open(path).bufferedReader().use { it.readText() }
        } catch (error: IOException) {
            throw IllegalStateException("无法读取 Shell Bridge shim 资源：$path", error)
        }

    private fun requireConfig(params: JSONObject?): EditableRuntimeConfig {
        val configJson = params?.optJSONObject("config")
        return EditableRuntimeConfig.fromJsonOrNull(configJson)
            ?: throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "运行配置参数结构无效",
            )
    }

    private fun requireString(params: JSONObject?, key: String): String {
        val value = params?.optString(key)
        if (value.isNullOrEmpty()) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "缺少参数：$key",
            )
        }
        return value
    }

    private fun requireNativeBackRequestId(params: JSONObject?): String {
        val requestId = params?.opt("requestId")
        if (requestId !is String || requestId.isBlank() || requestId.length > 64) {
            throw BridgeException(
                ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
                "原生返回应答 requestId 无效",
            )
        }
        return requestId
    }

    private fun validationResultJson(result: RuntimeConfigValidator.Result): JSONObject {
        val fieldErrors = JSONObject()
        for ((field, messages) in result.fieldErrors) {
            fieldErrors.put(field, org.json.JSONArray(messages))
        }
        return JSONObject()
            .put("valid", result.valid)
            .put("fieldErrors", fieldErrors)
    }

    private fun applyResultJson(
        validation: RuntimeConfigValidator.Result,
        applied: Boolean,
        snapshot: JSONObject,
        error: ShellError?,
    ): JSONObject {
        val result = validationResultJson(validation)
        result.put("applied", applied)
        result.put("snapshot", snapshot)
        if (error != null) {
            val errorJson =
                JSONObject()
                    .put("code", error.code)
                    .put("message", error.message)
            if (error.field != null) {
                errorJson.put("field", error.field)
            }
            result.put("error", errorJson)
        }
        return result
    }

    private fun isTrustedOrigin(origin: Uri): Boolean =
        origin.toString().trimEnd('/') == TRUSTED_ORIGIN.trimEnd('/')

    private fun isTrustedPageUrl(url: String?): Boolean {
        val uri = try {
            Uri.parse(url ?: return false)
        } catch (_: Exception) {
            return false
        }
        return uri.scheme.equals("https", ignoreCase = true) &&
            uri.host.equals("winestock.internal", ignoreCase = true) &&
            uri.port == -1
    }

    /** 稳定错误码 + 可选字段的运行错误。 */
    private data class ShellError(val code: String, val message: String, val field: String?)

    /** 分发过程中的可映射错误，携带稳定错误码。 */
    private class BridgeException(val code: String, message: String) : Exception(message)

    companion object {
        /** 注入到 window 的原生消息通道对象名，需与 bridge.js 的 channelName 默认值一致。 */
        const val CHANNEL_NAME = "__winestockShellBridgeNative__"

        /** 前端打包资源与桥消息共同信任的本地 origin。 */
        const val TRUSTED_ORIGIN = "https://winestock.internal"

        private const val SHIM_ASSET_PATH = "shell/bridge.js"

        private val NATIVE_BACK_REASONS =
            setOf(
                "transient-overlay",
                "image-preview",
                "dialog",
                "busy-dialog",
                "drawer",
                "popover",
                "page-state",
                "route-history",
                "handler-error",
                "unhandled",
            )
    }
}
