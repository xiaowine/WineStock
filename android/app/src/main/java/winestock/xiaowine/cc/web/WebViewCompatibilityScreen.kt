package winestock.xiaowine.cc.web

import android.content.res.Configuration
import android.view.View
import androidx.activity.ComponentActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import winestock.xiaowine.cc.R
import winestock.xiaowine.cc.databinding.ActivityWebviewCompatibilityBinding

/** 不依赖 WebView 的致命兼容性恢复页，只展示诊断信息并提供手动复检。 */
internal class WebViewCompatibilityScreen(
    private val activity: ComponentActivity,
    private val recheck: () -> WebViewCompatibilityResult,
    private val onSupported: () -> Unit,
) {
    private val binding = ActivityWebviewCompatibilityBinding.inflate(activity.layoutInflater)

    init {
        activity.setContentView(binding.root)
        installInsets()
        binding.retryButton.setOnClickListener { performRecheck() }
    }

    fun show(result: WebViewCompatibilityResult.Unsupported) {
        val provider = result.provider
        binding.compatibilityTitle.setText(WebViewCompatibilityPresentation.title(result.reason))
        binding.compatibilityResultLabel.setText(
            WebViewCompatibilityPresentation.resultLabel(result.reason),
        )
        binding.compatibilityMessage.text =
            activity.getString(WebViewCompatibilityPresentation.message(result.reason)) +
                "\n" +
                activity.getString(
                    R.string.webview_compatibility_diagnostic_code,
                    result.diagnosticCode
                        ?: WebViewCompatibilityPresentation.diagnosticCode(result.reason),
                )
        binding.requirementLabel.setText(
            WebViewCompatibilityPresentation.requirementLabel(result.reason),
        )
        binding.requirementValue.setText(
            WebViewCompatibilityPresentation.requirementValue(result.reason),
        )
        binding.currentProviderValue.text =
            provider?.let {
                activity.getString(
                    R.string.webview_compatibility_current_provider_value,
                    it.packageName,
                    it.versionName ?: activity.getString(R.string.webview_compatibility_unknown),
                )
            } ?: activity.getString(R.string.webview_compatibility_provider_unavailable)
    }

    fun onConfigurationChanged(newConfig: Configuration) {
        activity.window.setBackgroundDrawableResource(R.color.compatibility_background)
        val primaryText = ContextCompat.getColor(activity, R.color.compatibility_text_primary)
        val secondaryText = ContextCompat.getColor(activity, R.color.compatibility_text_secondary)
        listOf(
            binding.compatibilityTitle,
            binding.compatibilityResultLabel,
            binding.currentProviderValue,
            binding.requirementValue,
        ).forEach { it.setTextColor(primaryText) }
        listOf(
            binding.compatibilityMessage,
            binding.currentProviderLabel,
            binding.requirementLabel,
        ).forEach { it.setTextColor(secondaryText) }
        binding.retryButton.backgroundTintList =
            ContextCompat.getColorStateList(activity, R.color.compatibility_action_background)
        binding.retryButton.setTextColor(
            ContextCompat.getColor(activity, R.color.compatibility_action_text),
        )
        binding.brandMark.setImageResource(R.drawable.ic_brand_mark)
        val darkMode =
            newConfig.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        androidx.core.view.WindowInsetsControllerCompat(activity.window, binding.root).apply {
            isAppearanceLightStatusBars = !darkMode
            isAppearanceLightNavigationBars = !darkMode
        }
    }

    fun destroy() {
        ViewCompat.setOnApplyWindowInsetsListener(binding.content, null)
    }

    private fun performRecheck() {
        when (val result = recheck()) {
            is WebViewCompatibilityResult.Supported -> onSupported()
            is WebViewCompatibilityResult.Unsupported -> show(result)
        }
    }

    private fun installInsets() {
        val initialLeft = binding.content.paddingLeft
        val initialTop = binding.content.paddingTop
        val initialRight = binding.content.paddingRight
        val initialBottom = binding.content.paddingBottom
        ViewCompat.setOnApplyWindowInsetsListener(binding.content) { view, insets ->
            val safeInsets =
                insets.getInsets(
                    WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
                )
            view.setPadding(
                initialLeft + safeInsets.left,
                initialTop + safeInsets.top,
                initialRight + safeInsets.right,
                initialBottom + safeInsets.bottom,
            )
            insets
        }
        ViewCompat.requestApplyInsets(binding.content)
        onConfigurationChanged(activity.resources.configuration)
    }

}
