package winestock.xiaowine.cc.web

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.net.Uri
import android.view.View
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.core.content.ContextCompat
import androidx.webkit.WebViewAssetLoader
import androidx.webkit.WebSettingsCompat
import androidx.webkit.WebViewFeature
import winestock.xiaowine.cc.R

/**
 * 配置 WineStock shell WebView：受信任 asset 加载、系统栏钩子、文件选择、页面生命周期回调。
 * 不拥有 Shell Bridge 安装与业务协议。
 */
internal class ShellWebViewConfigurator(
    private val context: Context,
    private val assetLoader: WebViewAssetLoader,
    private val systemBarAppearance: SystemBarAppearanceController,
    private val fileChooserHost: WebViewFileChooserHost,
    private val cameraPermissionHost: WebViewCameraPermissionHost,
    private val onPageStarted: (url: String?) -> Unit,
    private val onPageVisible: (url: String?) -> Unit,
    private val onFrontendReady: () -> Unit,
    private val onRendererExit: (view: WebView, detail: RenderProcessGoneDetail) -> Boolean,
) {
    @SuppressLint("SetJavaScriptEnabled")
    fun configure(webView: WebView) {
        webView.apply {
            // WebView 在内容绘制前默认黑底；设为中性背景与启动窗口衔接，消除黑闪。
            setBackgroundColor(ContextCompat.getColor(context, R.color.web_background))
            // 关闭 WebView 原生 overscroll；滚动边界反馈交由前端呈现。
            overScrollMode = View.OVER_SCROLL_NEVER
            systemBarAppearance.installJavascriptInterface(this)
            webViewClient =
                object : WebViewClient() {
                    override fun shouldInterceptRequest(
                        view: WebView,
                        request: WebResourceRequest,
                    ): WebResourceResponse? = assetLoader.shouldInterceptRequest(request.url)

                    override fun onPageStarted(view: WebView?, url: String?, favicon: Bitmap?) {
                        super.onPageStarted(view, url, favicon)
                        onPageStarted(url)
                    }

                    override fun onPageCommitVisible(view: WebView?, url: String?) {
                        super.onPageCommitVisible(view, url)
                        onPageVisible(url)
                    }

                    override fun onPageFinished(view: WebView?, url: String?) {
                        super.onPageFinished(view, url)
                        onPageVisible(url)
                        // 兜底：真正的就绪信号来自前端，但 SPA 首屏未绘制时也不应长期保持启动画面。
                        onFrontendReady()
                    }

                    override fun onRenderProcessGone(
                        view: WebView,
                        detail: RenderProcessGoneDetail,
                    ): Boolean = onRendererExit(view, detail)
                }
            // HTML <input type="file">：系统选择器返回 content URI，无存储权限。
            // getUserMedia 摄像头：仅受信任 origin 且原生 CAMERA 已获准时放行，见 cameraPermissionHost。
            webChromeClient =
                object : WebChromeClient() {
                    override fun onShowFileChooser(
                        webView: WebView?,
                        filePathCallback: ValueCallback<Array<Uri>>?,
                        fileChooserParams: FileChooserParams?,
                    ): Boolean {
                        if (filePathCallback == null) return false
                        return fileChooserHost.onShowFileChooser(filePathCallback, fileChooserParams)
                    }

                    override fun onPermissionRequest(request: PermissionRequest?) {
                        if (request == null) return
                        cameraPermissionHost.onPermissionRequest(request)
                    }

                    override fun onPermissionRequestCanceled(request: PermissionRequest?) {
                        cameraPermissionHost.onPermissionRequestCanceled(request)
                    }
                }
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                // Android shell 是应用界面，不允许 WebView 对整页执行双指缩放；
                // 页面内画布等局部缩放仍由前端自己的 pointer/touch 逻辑负责。
                setSupportZoom(false)
                builtInZoomControls = false
                displayZoomControls = false
                // 前端已有完整双主题；禁用 WebView 算法着色，避免在深色 CSS 上再次反色。
                if (WebViewFeature.isFeatureSupported(WebViewFeature.ALGORITHMIC_DARKENING)) {
                    WebSettingsCompat.setAlgorithmicDarkeningAllowed(this, false)
                }
                // 前端在 https://winestock.internal，远端模式常连局域网 HTTP；明文范围由 network_security_config 控制。
                mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
            }
            // 前台可见时降低 renderer 被低内存回收的概率；不可见时仍允许系统降低其优先级。
            setRendererPriorityPolicy(WebView.RENDERER_PRIORITY_IMPORTANT, true)
        }
    }
}
