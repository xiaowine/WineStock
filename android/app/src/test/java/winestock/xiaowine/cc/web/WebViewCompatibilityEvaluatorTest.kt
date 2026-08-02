package winestock.xiaowine.cc.web

import androidx.webkit.WebViewFeature
import winestock.xiaowine.cc.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebViewCompatibilityEvaluatorTest {
    private val allFeatures =
        setOf(
            WebViewFeature.WEB_MESSAGE_LISTENER,
            WebViewFeature.DOCUMENT_START_SCRIPT,
        )

    @Test
    fun `missing provider is rejected`() {
        val result = WebViewCompatibilityEvaluator.evaluate(null, allFeatures)

        assertUnsupported(result, WebViewIncompatibilityReason.PROVIDER_UNAVAILABLE)
    }

    @Test
    fun `major version below 111 is rejected`() {
        val result = evaluate("110.0.5481.154", allFeatures)

        assertUnsupported(result, WebViewIncompatibilityReason.VERSION_TOO_OLD)
    }

    @Test
    fun `major version 111 with all required features is accepted`() {
        val result = evaluate("111.0.5563.58", allFeatures)

        assertTrue(result is WebViewCompatibilityResult.Supported)
        assertEquals(111, (result as WebViewCompatibilityResult.Supported).majorVersion)
    }

    @Test
    fun `new provider missing a required feature is rejected`() {
        val result = evaluate("149.0.7827.163", setOf(WebViewFeature.WEB_MESSAGE_LISTENER))

        val unsupported = assertUnsupported(
            result,
            WebViewIncompatibilityReason.REQUIRED_FEATURES_MISSING,
        )
        assertEquals(setOf(WebViewFeature.DOCUMENT_START_SCRIPT), unsupported.missingFeatures)
    }

    @Test
    fun `malformed version is rejected`() {
        val result = evaluate("version-149", allFeatures)

        assertUnsupported(result, WebViewIncompatibilityReason.VERSION_UNREADABLE)
    }

    @Test
    fun `version parser accepts provider suffix after numeric major`() {
        assertEquals(116, WebViewCompatibilityEvaluator.parseMajorVersion(" 116.0.5845.92 beta "))
        assertEquals(null, WebViewCompatibilityEvaluator.parseMajorVersion(null))
        assertEquals(null, WebViewCompatibilityEvaluator.parseMajorVersion(""))
    }

    @Test
    fun `each incompatibility reason has its own presentation mapping`() {
        val reasons = WebViewIncompatibilityReason.values()

        reasons.forEach { reason ->
            assertTrue(WebViewCompatibilityPresentation.diagnosticCode(reason).isNotEmpty())
            assertTrue(WebViewCompatibilityPresentation.title(reason) != 0)
            assertTrue(WebViewCompatibilityPresentation.message(reason) != 0)
            assertTrue(WebViewCompatibilityPresentation.resultLabel(reason) != 0)
        }
    }

    @Test
    fun `only bridge failure replaces the WebView requirement with failure stage`() {
        assertEquals(
            R.string.webview_compatibility_requirement_label,
            WebViewCompatibilityPresentation.requirementLabel(
                WebViewIncompatibilityReason.VERSION_TOO_OLD,
            ),
        )
        assertEquals(
            R.string.webview_compatibility_failure_stage_label,
            WebViewCompatibilityPresentation.requirementLabel(
                WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE,
            ),
        )
    }

    @Test
    fun `frontend bridge diagnostic codes are allowlisted`() {
        assertEquals(
            "SHELL_BRIDGE_READY_FAILED",
            WebViewCompatibilityPresentation.normalizeBridgeDiagnosticCode(
                "shell_bridge_ready_failed",
            ),
        )
        assertEquals(
            "SHELL_BRIDGE_UNAVAILABLE",
            WebViewCompatibilityPresentation.normalizeBridgeDiagnosticCode("unknown-code"),
        )
    }

    private fun evaluate(
        versionName: String,
        features: Set<String>,
    ): WebViewCompatibilityResult =
        WebViewCompatibilityEvaluator.evaluate(
            WebViewProviderSnapshot("com.example.webview", versionName),
            features,
        )

    private fun assertUnsupported(
        result: WebViewCompatibilityResult,
        reason: WebViewIncompatibilityReason,
    ): WebViewCompatibilityResult.Unsupported {
        assertTrue(result is WebViewCompatibilityResult.Unsupported)
        return (result as WebViewCompatibilityResult.Unsupported).also {
            assertEquals(reason, it.reason)
        }
    }
}
