package winestock.xiaowine.cc

import android.annotation.SuppressLint
import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.enableEdgeToEdge
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.webkit.WebViewAssetLoader
import winestock.xiaowine.cc.databinding.ActivityMainBinding
import winestock.xiaowine.cc.shell.ShellBridgeHost
import winestock.xiaowine.cc.web.FrontendPathHandler

/**
 * WineStock Android shell 的唯一 Activity。
 *
 * 职责：创建并配置 WebView、通过 WebViewAssetLoader 从受信任 origin 加载打包前端、
 * 在加载前安装 Shell Bridge、管理冷启动 SplashScreen 的保持与隐藏、处理返回键、在恢复时通知桥。
 * 它不渲染运行设置或业务 UI，不实现本地 Axum 服务，配置与业务能力分别由前端经 Shell Bridge 和 HTTP 使用。
 */
class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var assetLoader: WebViewAssetLoader
    private var shellBridge: ShellBridgeHost? = null

    /** 前端首屏是否就绪；SplashScreen 在此之前一直保持，就绪后系统冷启动窗口才退场。 */
    private var frontendReady = false

    private val mainHandler = Handler(Looper.getMainLooper())

    /** 无就绪信号时的兜底：超时后放行 SplashScreen，避免异常时永久卡在启动画面。 */
    private val splashTimeout = Runnable { markFrontendReady() }

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        // 必须在 super.onCreate 前接管系统启动窗口。
        val splashScreen = installSplashScreen()
        super.onCreate(savedInstanceState)
        // 前端就绪前保持冷启动画面；就绪或超时后系统自动隐藏并按主题退场。
        splashScreen.setKeepOnScreenCondition { !frontendReady }

        enableEdgeToEdge()
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        applyStatusBarAppearance()
        applySystemBarInsets()

        assetLoader =
            WebViewAssetLoader.Builder()
                .setDomain(AppConfig.TRUSTED_HOST)
                .addPathHandler("/", FrontendPathHandler(this))
                .build()

        configureWebView()
        installShellBridge()
        installBackNavigation()

        mainHandler.postDelayed(splashTimeout, AppConfig.SPLASH_TIMEOUT_MS)
        binding.webView.loadUrl(AppConfig.FRONTEND_HOME_URL)
    }

    private fun applyStatusBarAppearance() {
        val isNightMode = (resources.configuration.uiMode and
            Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        WindowInsetsControllerCompat(window, binding.root).apply {
            // 浅色背景用深色图标，深色背景用浅色图标。
            isAppearanceLightStatusBars = !isNightMode
            isAppearanceLightNavigationBars = !isNightMode
        }
    }

    private fun applySystemBarInsets() {
        ViewCompat.setOnApplyWindowInsetsListener(binding.main) { v, insets ->
            val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
            v.setPadding(systemBars.left, systemBars.top, systemBars.right, systemBars.bottom)
            insets
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun configureWebView() {
        binding.webView.apply {
            // WebView 在内容绘制前默认黑底；设为中性背景与启动窗口衔接，消除黑闪。
            setBackgroundColor(ContextCompat.getColor(this@MainActivity, R.color.web_background))
            // 关闭 WebView 原生 overscroll（拖到顶/底的拉伸果冻或发光回弹）；
            // 滚动边界反馈交由前端页面自身呈现，避免原生效果叠加在 Web 内容之上。
            overScrollMode = View.OVER_SCROLL_NEVER
            webViewClient = object : WebViewClient() {
                override fun shouldInterceptRequest(
                    view: WebView,
                    request: WebResourceRequest,
                ): WebResourceResponse? = assetLoader.shouldInterceptRequest(request.url)

                override fun onPageFinished(view: WebView?, url: String?) {
                    super.onPageFinished(view, url)
                    // 兜底：真正的就绪信号来自前端，但 SPA 首屏未绘制时也不应长期保持启动画面。
                    markFrontendReady()
                }
            }
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                // 前端页面运行在 https://winestock.internal（secure context），但远端模式常连
                // 局域网 HTTP 服务器；放开 mixed content 使 https 页面能请求 http API。
                // 明文范围由 network_security_config.xml 控制。
                mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
            }
        }
    }

    /** 在加载前端前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。 */
    private fun installShellBridge() {
        val bridge =
            ShellBridgeHost(
                context = this,
                deviceName = resolveDeviceName(),
                appVersion = resolveAppVersion(),
                // 前端首屏就绪的准确信号，早于或替代 onPageFinished 兜底。
                onFrontendReady = { markFrontendReady() },
            )
        if (bridge.install(binding.webView)) {
            shellBridge = bridge
        }
        // 桥不可用时前端会通过降级桥进入可修复失败态，Activity 仍加载前端资源。
    }

    /** 标记前端就绪并清除兜底超时；幂等。SplashScreen 会在下一帧检测到条件变化后退场。 */
    private fun markFrontendReady() {
        if (frontendReady) return
        frontendReady = true
        mainHandler.removeCallbacks(splashTimeout)
    }

    private fun installBackNavigation() {
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (binding.webView.canGoBack()) {
                    binding.webView.goBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                }
            }
        })
    }

    override fun onResume() {
        super.onResume()
        // 通知前端应用恢复，触发服务可用性补检；桥未安装时为 no-op。
        shellBridge?.notifyAppResumed()
    }

    override fun onDestroy() {
        mainHandler.removeCallbacks(splashTimeout)
        super.onDestroy()
    }

    private fun resolveDeviceName(): String {
        val manufacturer = Build.MANUFACTURER?.replaceFirstChar { it.uppercase() }.orEmpty()
        val model = Build.MODEL.orEmpty()
        val label = listOf(manufacturer, model).filter { it.isNotBlank() }.joinToString(" ")
        return label.ifBlank { "WineStock Android" }
    }

    private fun resolveAppVersion(): String =
        try {
            packageManager.getPackageInfo(packageName, 0).versionName ?: "unknown"
        } catch (_: Exception) {
            "unknown"
        }
}
