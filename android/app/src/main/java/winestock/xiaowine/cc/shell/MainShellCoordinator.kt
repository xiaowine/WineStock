package winestock.xiaowine.cc.shell

import android.content.Intent
import android.content.res.Configuration
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
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
) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val splashGate =
        SplashFrontendGate(
            mainHandler = mainHandler,
            timeoutMs = AppConfig.SPLASH_TIMEOUT_MS,
        )
    private val fileChooserHost = WebViewFileChooserHost(launchIntent = launchFileChooser)
    private val cameraPermissionHost =
        WebViewCameraPermissionHost(
            context = activity,
            trustedOrigin = AppConfig.TRUSTED_ORIGIN,
            requestCameraPermission = requestCameraPermission,
        )

    private lateinit var binding: ActivityMainBinding
    private lateinit var systemBarAppearance: SystemBarAppearanceController
    private var shellBridge: ShellBridgeHost? = null
    private var viewportInsetsPublisher: WebViewportInsetsPublisher? = null

    /**
     * 完成 edge-to-edge、内容视图、WebView、Bridge 与返回协商，并加载前端入口。
     * 调用前 Activity 须已 `super.onCreate`。
     */
    fun start(splashScreen: SplashScreen) {
        splashGate.attach(splashScreen)

        activity.enableEdgeToEdge()
        binding = ActivityMainBinding.inflate(activity.layoutInflater)
        activity.setContentView(binding.root)

        systemBarAppearance =
            SystemBarAppearanceController(
                activity = activity,
                contentView = { binding.root },
                mainHandler = mainHandler,
            ).also { it.applyDefaultBars() }

        viewportInsetsPublisher =
            WebViewportInsetsPublisher(
                insetTarget = binding.root,
                webView = binding.webView,
                trustedOrigin = AppConfig.TRUSTED_ORIGIN,
            ).also { it.install() }

        val assetLoader =
            WebViewAssetLoader.Builder()
                .setDomain(AppConfig.TRUSTED_HOST)
                .addPathHandler("/", FrontendPathHandler(activity))
                .build()

        ShellWebViewConfigurator(
            context = activity,
            assetLoader = assetLoader,
            systemBarAppearance = systemBarAppearance,
            fileChooserHost = fileChooserHost,
            cameraPermissionHost = cameraPermissionHost,
            onPageStarted = { url -> shellBridge?.onPageStarted(url) },
            onPageVisible = { url -> viewportInsetsPublisher?.onPageVisible(url) },
            onFrontendReady = { splashGate.markReady() },
        ).configure(binding.webView)

        installShellBridge()
        NativeBackNavigator(
            activity = activity,
            webView = { binding.webView },
            shellBridge = { shellBridge },
        ).install(activity.onBackPressedDispatcher)

        binding.webView.loadUrl(AppConfig.FRONTEND_HOME_URL)
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
        binding.webView.setBackgroundColor(backgroundColor)
        systemBarAppearance.onConfigurationChanged(newConfig)
        viewportInsetsPublisher?.refresh()
        // WebView 会更新 CSS media query，但部分版本不派发 MediaQueryList change；前端需主动重读。
        binding.webView.post {
            if (activity.isFinishing || activity.isDestroyed) return@post
            runCatching {
                binding.webView.evaluateJavascript(SYSTEM_THEME_REFRESH_SCRIPT, null)
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
        shellBridge?.destroy()
        shellBridge = null
        viewportInsetsPublisher?.dispose()
        viewportInsetsPublisher = null
    }

    /** 在加载前端前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。 */
    private fun installShellBridge() {
        val bridge =
            ShellBridgeHost(
                context = activity,
                runtimeManager =
                    (activity.application as WineStockApplication).localCoreRuntimeManager,
                deviceName = DeviceMetadata.resolveDeviceName(),
                appVersion = DeviceMetadata.resolveAppVersion(activity),
                nativeBackResponseTimeoutMs = AppConfig.NATIVE_BACK_RESPONSE_TIMEOUT_MS,
                onFrontendReady = { splashGate.markReady() },
            )
        if (bridge.install(binding.webView)) {
            shellBridge = bridge
        } else {
            bridge.destroy()
        }
        // 桥不可用时前端会通过降级桥进入可修复失败态，Activity 仍加载前端资源。
    }

    companion object {
        private const val SYSTEM_THEME_REFRESH_SCRIPT =
            "window.dispatchEvent(new Event(\"winestock:system-theme-refresh\"));"
    }
}
