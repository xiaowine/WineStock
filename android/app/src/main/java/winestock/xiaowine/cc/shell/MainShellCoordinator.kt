package winestock.xiaowine.cc.shell

import android.content.Intent
import android.content.res.Configuration
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.ViewGroup
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResult
import androidx.core.content.ContextCompat
import androidx.core.splashscreen.SplashScreen
import androidx.webkit.WebViewAssetLoader
import winestock.xiaowine.cc.AppConfig
import winestock.xiaowine.cc.R
import winestock.xiaowine.cc.WineStockApplication
import winestock.xiaowine.cc.databinding.ActivityMainBinding
import winestock.xiaowine.cc.web.FrontendPathHandler
import winestock.xiaowine.cc.web.ShellWebViewConfigurator
import winestock.xiaowine.cc.web.SplashFrontendGate
import winestock.xiaowine.cc.web.SystemBarAppearanceController
import winestock.xiaowine.cc.web.WebViewCameraPermissionHost
import winestock.xiaowine.cc.web.WebViewFileChooserHost
import winestock.xiaowine.cc.web.WebViewportInsetsPublisher

/**
 * MainActivity 的 shell 组装与生命周期协调器。
 *
 * 拥有 WebView、Bridge、安全区、Splash、系统栏外观与文件选择的接线；
 * Activity 只保留系统回调入口与 [registerForActivityResult] 注册。
 */
internal class MainShellCoordinator(
    private val activity: ComponentActivity,
    launchFileChooser: (Intent) -> Unit,
    requestCameraPermission: () -> Unit,
    private val forceBridgeInstallFailure: Boolean = false,
    private val forceBridgeHandshakeFailure: Boolean = false,
    private val onBridgeFailure: (String) -> Unit = {},
) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val splashGate = SplashFrontendGate()
    private val fileChooserHost = WebViewFileChooserHost(launchIntent = launchFileChooser)
    private val cameraPermissionHost =
        WebViewCameraPermissionHost(
            context = activity,
            trustedOrigin = AppConfig.TRUSTED_ORIGIN,
            requestCameraPermission = requestCameraPermission,
        )

    private lateinit var binding: ActivityMainBinding
    private lateinit var webView: WebView
    private lateinit var assetLoader: WebViewAssetLoader
    private lateinit var systemBarAppearance: SystemBarAppearanceController
    private var shellBridge: ShellBridgeHost? = null
    private var viewportInsetsPublisher: WebViewportInsetsPublisher? = null
    private var shellFailureHandled = false

    /**
     * 完成 edge-to-edge、内容视图、WebView、Bridge 与返回协商，并加载前端入口。
     * 调用前 Activity 须已 `super.onCreate`。
     */
    fun start(splashScreen: SplashScreen? = null) {
        splashScreen?.let(splashGate::attach)

        binding = ActivityMainBinding.inflate(activity.layoutInflater)
        activity.setContentView(binding.root)

        systemBarAppearance =
            SystemBarAppearanceController(
                activity = activity,
                contentView = { binding.root },
                mainHandler = mainHandler,
            ).also { it.applyDefaultBars() }

        assetLoader = createAssetLoader()
        webView = binding.webView
        configureWebView(webView)
        if (shellFailureHandled) return
        NativeBackNavigator(
            activity = activity,
            webView = { webView },
            shellBridge = { shellBridge },
        ).install(activity.onBackPressedDispatcher)
    }

    fun onFileChooserResult(result: ActivityResult) {
        fileChooserHost.onActivityResult(result)
    }

    fun onCameraPermissionResult(granted: Boolean) {
        cameraPermissionHost.onNativePermissionResult(granted)
    }

    fun onResume() {
        shellBridge?.onActivityResumed()
        viewportInsetsPublisher?.refresh()
        systemBarAppearance.reapply()
        shellBridge?.notifyAppResumed()
    }

    /**
     * 原地应用系统 day/night 配置，不重新创建 WebView；前端通过 media query 自行切换主题。
     * 原生层只刷新页面背后的系统主题表面、系统栏与配置相关的安全区计算。
     */
    fun onConfigurationChanged(newConfig: Configuration) {
        val backgroundColor = ContextCompat.getColor(activity, R.color.web_background)
        activity.window.setBackgroundDrawableResource(R.color.web_background)
        binding.root.setBackgroundColor(backgroundColor)
        webView.setBackgroundColor(backgroundColor)
        systemBarAppearance.onConfigurationChanged(newConfig)
        viewportInsetsPublisher?.refresh()
        // WebView 会更新 CSS media query，但部分版本不派发 MediaQueryList change；前端需主动重读。
        webView.post {
            if (activity.isFinishing || activity.isDestroyed) return@post
            runCatching {
                webView.evaluateJavascript(SYSTEM_THEME_REFRESH_SCRIPT, null)
            }
        }
    }

    fun onPause() {
        shellBridge?.onActivityPaused()
    }

    fun onStop() {
        shellBridge?.onActivityPaused()
    }

    fun onDestroy() {
        splashGate.cancelTimeout()
        fileChooserHost.destroy()
        cameraPermissionHost.destroy()
        disposeWebViewInfrastructure()
        if (::webView.isInitialized && !shellFailureHandled) {
            binding.webViewContainer.removeView(webView)
            webView.destroy()
        }
    }

    /** Bridge 无法安装或前端契约不兼容时，销毁 WebView 并把恢复 UI 交回 Activity。 */
    fun stopForBridgeFailure(message: String) {
        if (shellFailureHandled) return
        shellFailureHandled = true
        splashGate.markReady()
        fileChooserHost.cancelPending()
        cameraPermissionHost.cancelPending()
        disposeWebViewInfrastructure()
        if (::webView.isInitialized) {
            binding.webViewContainer.removeView(webView)
            webView.destroy()
        }
        onBridgeFailure(message)
    }

    private fun configureWebView(target: WebView) {
        viewportInsetsPublisher =
            WebViewportInsetsPublisher(
                insetSource = binding.root,
                imeInsetTarget = binding.webViewContainer,
                webView = target,
                trustedOrigin = AppConfig.TRUSTED_ORIGIN,
            ).also { it.install() }
        ShellWebViewConfigurator(
            context = activity,
            assetLoader = assetLoader,
            systemBarAppearance = systemBarAppearance,
            fileChooserHost = fileChooserHost,
            cameraPermissionHost = cameraPermissionHost,
            onPageStarted = { url -> shellBridge?.onPageStarted(url) },
            onPageVisible = { url -> viewportInsetsPublisher?.onPageVisible(url) },
            onRendererExit = ::recoverFromRendererExit,
        ).configure(target)
        installShellBridge(target)
        if (shellFailureHandled) return
        target.loadUrl(AppConfig.FRONTEND_HOME_URL)
    }

    /** Renderer 退出后的 WebView 不可复用；只重建 UI 链路，不重启 Application 级本地 core。 */
    private fun recoverFromRendererExit(
        failedWebView: WebView,
        detail: RenderProcessGoneDetail,
    ): Boolean {
        logRendererExit(detail)
        if (failedWebView !== webView) {
            failedWebView.destroy()
            return true
        }

        fileChooserHost.cancelPending()
        cameraPermissionHost.cancelPending()
        disposeWebViewInfrastructure()
        binding.webViewContainer.removeView(failedWebView)
        failedWebView.destroy()

        if (activity.isFinishing || activity.isDestroyed) return true

        WebView(activity).also { replacement ->
            webView = replacement
            binding.webViewContainer.addView(
                replacement,
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
            configureWebView(replacement)
        }
        return true
    }

    private fun disposeWebViewInfrastructure() {
        shellBridge?.destroy()
        shellBridge = null
        viewportInsetsPublisher?.dispose()
        viewportInsetsPublisher = null
    }

    private fun createAssetLoader(): WebViewAssetLoader =
        WebViewAssetLoader.Builder()
            .setDomain(AppConfig.TRUSTED_HOST)
            .addPathHandler("/", FrontendPathHandler(activity))
            .build()

    private fun logRendererExit(detail: RenderProcessGoneDetail) {
        val provider = WebView.getCurrentWebViewPackage()
        Log.e(
            LOG_TAG,
            "WebView renderer exited: didCrash=${detail.didCrash()}, " +
                "priorityAtExit=${detail.rendererPriorityAtExit()}, " +
                "provider=${provider?.packageName ?: "unavailable"}, " +
                "version=${provider?.versionName ?: "unknown"}",
        )
    }

    /** 在加载前端前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。 */
    private fun installShellBridge(webView: WebView) {
        val bridge =
            ShellBridgeHost(
                context = activity,
                runtimeManager =
                    (activity.application as WineStockApplication).localCoreRuntimeManager,
                deviceName = DeviceMetadata.resolveDeviceName(),
                appVersion = DeviceMetadata.resolveAppVersion(activity),
                nativeBackResponseTimeoutMs = AppConfig.NATIVE_BACK_RESPONSE_TIMEOUT_MS,
                frontendReadyTimeoutMs = AppConfig.SHELL_BRIDGE_READY_TIMEOUT_MS,
                forceFrontendReadyFailure = forceBridgeHandshakeFailure,
                onFrontendReady = { splashGate.markReady() },
                onBridgeFailure = ::stopForBridgeFailure,
            )
        if (!forceBridgeInstallFailure && bridge.install(webView)) {
            shellBridge = bridge
        } else {
            bridge.destroy()
            stopForBridgeFailure("Shell Bridge 安装失败")
        }
    }

    companion object {
        private const val LOG_TAG = "WineStockWebView"
        private const val SYSTEM_THEME_REFRESH_SCRIPT =
            "window.dispatchEvent(new Event(\"winestock:system-theme-refresh\"));"
    }
}
