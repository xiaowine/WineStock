package winestock.xiaowine.cc.web

import winestock.xiaowine.cc.R

/**
 * Android WebView 启动阻断页的原因到资源映射。
 *
 * 只拥有用户可见的标题、正文和诊断标签，不执行 WebView 探测，也不决定恢复动作。
 */
internal object WebViewCompatibilityPresentation {
    fun diagnosticCode(reason: WebViewIncompatibilityReason): String =
        when (reason) {
            WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE -> "WEBVIEW_PROVIDER_UNAVAILABLE"
            WebViewIncompatibilityReason.VERSION_UNREADABLE -> "WEBVIEW_VERSION_UNREADABLE"
            WebViewIncompatibilityReason.VERSION_TOO_OLD -> "WEBVIEW_VERSION_TOO_OLD"
            WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING ->
                "WEBVIEW_REQUIRED_FEATURES_MISSING"
            WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE -> "SHELL_BRIDGE_UNAVAILABLE"
        }

    /** 只允许前端约定的稳定码进入用户提示，未知值统一降级。 */
    fun normalizeBridgeDiagnosticCode(code: String): String =
        when (code) {
            "shell_bridge_unavailable" -> "SHELL_BRIDGE_UNAVAILABLE"
            "shell_bridge_snapshot_invalid" -> "SHELL_BRIDGE_SNAPSHOT_INVALID"
            "shell_bridge_version_mismatch" -> "SHELL_BRIDGE_VERSION_MISMATCH"
            "shell_bridge_method_missing" -> "SHELL_BRIDGE_METHOD_MISSING"
            "shell_bridge_extension_invalid" -> "SHELL_BRIDGE_EXTENSION_INVALID"
            "shell_bridge_event_subscription_failed" ->
                "SHELL_BRIDGE_EVENT_SUBSCRIPTION_FAILED"
            "shell_bridge_ready_failed" -> "SHELL_BRIDGE_READY_FAILED"
            "frontend_load_timeout" -> "FRONTEND_LOAD_TIMEOUT"
            else -> "SHELL_BRIDGE_UNAVAILABLE"
        }

    fun title(reason: WebViewIncompatibilityReason): Int =
        when (reason) {
            WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE ->
                R.string.webview_compatibility_title_provider_unavailable
            WebViewIncompatibilityReason.VERSION_UNREADABLE ->
                R.string.webview_compatibility_title_version_unreadable
            WebViewIncompatibilityReason.VERSION_TOO_OLD ->
                R.string.webview_compatibility_title_version_too_old
            WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING ->
                R.string.webview_compatibility_title_features_missing
            WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE ->
                R.string.webview_compatibility_title_shell_bridge_unavailable
        }

    fun message(reason: WebViewIncompatibilityReason): Int =
        when (reason) {
            WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE ->
                R.string.webview_compatibility_message_provider_unavailable
            WebViewIncompatibilityReason.VERSION_UNREADABLE ->
                R.string.webview_compatibility_message_version_unreadable
            WebViewIncompatibilityReason.VERSION_TOO_OLD ->
                R.string.webview_compatibility_message_version_too_old
            WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING ->
                R.string.webview_compatibility_message_features_missing
            WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE ->
                R.string.webview_compatibility_message_shell_bridge_unavailable
        }

    fun resultLabel(reason: WebViewIncompatibilityReason): Int =
        when (reason) {
            WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE ->
                R.string.webview_compatibility_result_provider_unavailable
            WebViewIncompatibilityReason.VERSION_UNREADABLE ->
                R.string.webview_compatibility_result_version_unreadable
            WebViewIncompatibilityReason.VERSION_TOO_OLD ->
                R.string.webview_compatibility_result_version_too_old
            WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING ->
                R.string.webview_compatibility_result_features_missing
            WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE ->
                R.string.webview_compatibility_result_shell_bridge_unavailable
        }

    fun requirementLabel(reason: WebViewIncompatibilityReason): Int =
        if (reason == WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE) {
            R.string.webview_compatibility_failure_stage_label
        } else {
            R.string.webview_compatibility_requirement_label
        }

    fun requirementValue(reason: WebViewIncompatibilityReason): Int =
        if (reason == WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE) {
            R.string.webview_compatibility_failure_stage_value
        } else {
            R.string.webview_compatibility_requirement_value
        }
}
