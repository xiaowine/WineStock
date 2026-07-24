package winestock.xiaowine.cc.shell

import android.content.Intent
import android.os.Handler
import android.os.Looper
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import androidx.core.splashscreen.SplashScreen
import androidx.webkit.WebViewAssetLoader
import winestock.xiaowine.cc.AppConfig
import winestock.xiaowine.cc.WineStockApplication
import winestock.xiaowine.cc.databinding.ActivityMainBinding
import winestock.xiaowine.cc.web.FrontendPathHandler
import winestock.xiaowine.cc.web.ShellWebViewConfigurator
import winestock.xiaowine.cc.web.SplashFrontendGate
import winestock.xiaowine.cc.web.SystemBarAppearanceController
import winestock.xiaowine.cc.web.WebViewFileChooserHost
import winestock.xiaowine.cc.web.WebViewportInsetsPublisher

/**
 * MainActivity 的 shell 组装与生命周期协调器。
 *
 * 拥有 WebView、Bridge、安全区、Splash、系统栏外观与文件选择的接线；
 * Activity 只保留系统回调入口与 [registerForActivityResult] 注册。
 */
internal class MainShellCoordinator(
    private val activity: AppCompatActivity,
    launchFileChooser: (Intent) -> Unit,
) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val splashGate =
        SplashFrontendGate(
            mainHandler = mainHandler,
            timeoutMs = AppConfig.SPLASH_TIMEOUT_MS,
        )
    private val fileChooserHost = WebViewFileChooserHost(launchIntent = launchFileChooser)

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
            ).also { it.applyDefaultLightBars() }

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

    fun onResume() {
        shellBridge?.onActivityResumed()
        viewportInsetsPublisher?.refresh()
        systemBarAppearance.reapply()
        shellBridge?.notifyAppResumed()
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
}
