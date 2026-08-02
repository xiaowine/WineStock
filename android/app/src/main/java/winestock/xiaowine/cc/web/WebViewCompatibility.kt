package winestock.xiaowine.cc.web

import android.util.Log
import android.webkit.WebView
import androidx.webkit.WebViewFeature

internal data class WebViewProviderSnapshot(
    val packageName: String,
    val versionName: String?,
)

internal enum class WebViewIncompatibilityReason {
    PROVIDER_UNAVAILABLE,
    VERSION_UNREADABLE,
    VERSION_TOO_OLD,
    REQUIRED_FEATURES_MISSING,
    SHELL_BRIDGE_UNAVAILABLE,
}

internal sealed interface WebViewCompatibilityResult {
    val provider: WebViewProviderSnapshot?

    data class Supported(
        override val provider: WebViewProviderSnapshot,
        val majorVersion: Int,
    ) : WebViewCompatibilityResult

    data class Unsupported(
        override val provider: WebViewProviderSnapshot?,
        val reason: WebViewIncompatibilityReason,
        val missingFeatures: Set<String> = emptySet(),
        val diagnosticCode: String? = null,
    ) : WebViewCompatibilityResult
}

/** WebView 版本与 Shell Bridge 必需能力的纯判定边界。 */
internal object WebViewCompatibilityEvaluator {
    const val MINIMUM_MAJOR_VERSION = 111

    fun evaluate(
        provider: WebViewProviderSnapshot?,
        supportedFeatures: Set<String>,
    ): WebViewCompatibilityResult {
        if (provider == null) {
            return WebViewCompatibilityResult.Unsupported(
                provider = null,
                reason = WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE,
            )
        }

        val majorVersion = parseMajorVersion(provider.versionName)
            ?: return WebViewCompatibilityResult.Unsupported(
                provider = provider,
                reason = WebViewIncompatibilityReason.VERSION_UNREADABLE,
            )
        if (majorVersion < MINIMUM_MAJOR_VERSION) {
            return WebViewCompatibilityResult.Unsupported(
                provider = provider,
                reason = WebViewIncompatibilityReason.VERSION_TOO_OLD,
            )
        }

        val missingFeatures = REQUIRED_FEATURES - supportedFeatures
        if (missingFeatures.isNotEmpty()) {
            return WebViewCompatibilityResult.Unsupported(
                provider = provider,
                reason = WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING,
                missingFeatures = missingFeatures,
            )
        }
        return WebViewCompatibilityResult.Supported(provider, majorVersion)
    }

    fun parseMajorVersion(versionName: String?): Int? {
        val value = versionName?.trim().orEmpty()
        val leadingDigits = value.takeWhile(Char::isDigit)
        return leadingDigits.takeIf(String::isNotEmpty)?.toIntOrNull()
    }

    val REQUIRED_FEATURES: Set<String> =
        setOf(
            WebViewFeature.WEB_MESSAGE_LISTENER,
            WebViewFeature.DOCUMENT_START_SCRIPT,
        )
}

/** 在创建任何 WebView 实例前探测当前 provider；异常或未知状态一律关闭前端入口。 */
internal object WebViewCompatibility {
    fun check(): WebViewCompatibilityResult {
        val provider =
            runCatching { WebView.getCurrentWebViewPackage() }
                .onFailure { Log.e(LOG_TAG, "Unable to query WebView provider", it) }
                .getOrNull()
                ?.let { packageInfo ->
                    WebViewProviderSnapshot(
                        packageName = packageInfo.packageName,
                        versionName = packageInfo.versionName,
                    )
                }
        val supportedFeatures =
            WebViewCompatibilityEvaluator.REQUIRED_FEATURES.filterTo(mutableSetOf()) { feature ->
                runCatching { WebViewFeature.isFeatureSupported(feature) }
                    .onFailure { Log.e(LOG_TAG, "Unable to query WebView feature=$feature", it) }
                    .getOrDefault(false)
            }
        val result = WebViewCompatibilityEvaluator.evaluate(provider, supportedFeatures)
        Log.i(
            LOG_TAG,
            "WebView compatibility: provider=${provider?.packageName ?: "unavailable"}, " +
                "version=${provider?.versionName ?: "unknown"}, features=$supportedFeatures, " +
                "supported=${result is WebViewCompatibilityResult.Supported}",
        )
        return result
    }

    private const val LOG_TAG = "WineStockWebView"
}
