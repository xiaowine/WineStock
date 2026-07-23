package winestock.xiaowine.cc

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.core.view.WindowInsetsControllerCompat
import androidx.webkit.WebViewAssetLoader
import winestock.xiaowine.cc.databinding.ActivityMainBinding
import winestock.xiaowine.cc.shell.ShellBridgeHost
import winestock.xiaowine.cc.web.FrontendPathHandler
import winestock.xiaowine.cc.web.WebViewFileChooserSession
import winestock.xiaowine.cc.web.WebViewportInsetsPublisher

/**
 * WineStock Android shell 的唯一 Activity。
 *
 * 职责：创建并配置 WebView、通过 WebViewAssetLoader 从受信任 origin 加载打包前端、
 * 在加载前安装 Shell Bridge、保持 edge-to-edge 并向前端发布安全区、管理冷启动 SplashScreen、
 * 处理返回键并在恢复时通知桥、承接 HTML 文件选择（系统 SAF/chooser，无存储权限）。
 * 它不渲染运行设置或业务 UI，不实现本地 Axum 服务，配置与业务能力分别由前端经 Shell Bridge 和 HTTP 使用。
 */
class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var assetLoader: WebViewAssetLoader
    private var shellBridge: ShellBridgeHost? = null
    private var viewportInsetsPublisher: WebViewportInsetsPublisher? = null

    /**
     * WebView `<input type="file">` 的单 pending 回调会话。
     * Intent 启动在本 Activity；URI 所有权与一次结算在 [WebViewFileChooserSession]。
     */
    private val fileChooserSession = WebViewFileChooserSession()

    /**
     * 系统文件选择器结果入口。必须在 STARTED 前 register；结果经 session 映射后只结算一次 ValueCallback。
     */
    private val fileChooserLauncher =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val uriStrings =
                WebViewFileChooserSession.mapChooserResult(
                    resultOk = result.resultCode == Activity.RESULT_OK,
                    dataUri = result.data?.data?.toString(),
                    clipUris = extractClipUris(result.data),
                )
            fileChooserSession.deliver(uriStrings)
        }

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
        viewportInsetsPublisher =
            WebViewportInsetsPublisher(
                insetTarget = binding.root,
                webView = binding.webView,
                trustedOrigin = AppConfig.TRUSTED_ORIGIN,
            ).also { it.install() }

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
        WindowInsetsControllerCompat(window, binding.root).apply {
            // 前端当前固定为浅色主题，系统栏使用深色图标；不跟随系统夜间模式切换。
            isAppearanceLightStatusBars = true
            isAppearanceLightNavigationBars = true
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

                override fun onPageStarted(view: WebView?, url: String?, favicon: Bitmap?) {
                    super.onPageStarted(view, url, favicon)
                    shellBridge?.onPageStarted(url)
                }

                override fun onPageCommitVisible(view: WebView?, url: String?) {
                    super.onPageCommitVisible(view, url)
                    viewportInsetsPublisher?.onPageVisible(url)
                }

                override fun onPageFinished(view: WebView?, url: String?) {
                    super.onPageFinished(view, url)
                    viewportInsetsPublisher?.onPageVisible(url)
                    // 兜底：真正的就绪信号来自前端，但 SPA 首屏未绘制时也不应长期保持启动画面。
                    markFrontendReady()
                }
            }
            // HTML <input type="file"> 需要宿主实现 onShowFileChooser；不声明存储/媒体权限，
            // 仅用系统选择器返回的 content URI 临时授权完成上传。
            webChromeClient =
                object : WebChromeClient() {
                    override fun onShowFileChooser(
                        webView: WebView?,
                        filePathCallback: ValueCallback<Array<Uri>>?,
                        fileChooserParams: FileChooserParams?,
                    ): Boolean {
                        if (filePathCallback == null) return false
                        return launchSystemFileChooser(filePathCallback, fileChooserParams)
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

    /**
     * 登记 pending ValueCallback 并启动系统文件选择器。
     * 返回 true 表示宿主已接管选择流程（含启动失败时的 null 结算），WebView 不再走默认路径。
     */
    private fun launchSystemFileChooser(
        filePathCallback: ValueCallback<Array<Uri>>,
        fileChooserParams: WebChromeClient.FileChooserParams?,
    ): Boolean {
        val accepted =
            fileChooserSession.begin { uriStrings ->
                val uris = uriStrings?.map(Uri::parse)?.toTypedArray()
                filePathCallback.onReceiveValue(uris)
            }
        if (!accepted) {
            // session 已 destroy：begin 已对 WebView 回调 null，勿再 launch。
            return true
        }

        val intent =
            try {
                fileChooserParams?.createIntent()
                    ?: Intent(Intent.ACTION_GET_CONTENT).apply {
                        addCategory(Intent.CATEGORY_OPENABLE)
                        type = "*/*"
                    }
            } catch (_: Exception) {
                fileChooserSession.cancel()
                return true
            }

        return try {
            fileChooserLauncher.launch(intent)
            true
        } catch (_: ActivityNotFoundException) {
            fileChooserSession.cancel()
            true
        } catch (_: Exception) {
            fileChooserSession.cancel()
            true
        }
    }

    /** 从选择器 Intent 提取多选 ClipData URI；无 ClipData 时返回空列表，交 mapChooserResult 使用 data URI。 */
    private fun extractClipUris(data: Intent?): List<String> {
        val clip = data?.clipData ?: return emptyList()
        val uris = ArrayList<String>(clip.itemCount)
        for (index in 0 until clip.itemCount) {
            val uri = clip.getItemAt(index)?.uri?.toString()
            if (!uri.isNullOrBlank()) {
                uris += uri
            }
        }
        return uris
    }

    /** 在加载前端前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。 */
    private fun installShellBridge() {
        val bridge =
            ShellBridgeHost(
                context = this,
                runtimeManager =
                    (application as WineStockApplication).localCoreRuntimeManager,
                deviceName = resolveDeviceName(),
                appVersion = resolveAppVersion(),
                nativeBackResponseTimeoutMs = AppConfig.NATIVE_BACK_RESPONSE_TIMEOUT_MS,
                // 前端首屏就绪的准确信号，早于或替代 onPageFinished 兜底。
                onFrontendReady = { markFrontendReady() },
            )
        if (bridge.install(binding.webView)) {
            shellBridge = bridge
        } else {
            bridge.destroy()
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
                when (
                    shellBridge?.requestNativeBack(binding.webView.canGoBack()) { handled ->
                        if (!handled) performNativeFallback(this)
                    }
                ) {
                    ShellBridgeHost.NativeBackDispatchResult.DISPATCHED,
                    ShellBridgeHost.NativeBackDispatchResult.ALREADY_PENDING -> return
                    ShellBridgeHost.NativeBackDispatchResult.UNAVAILABLE,
                    null -> performNativeFallback(this)
                }
            }
        })
    }

    /** 协商未处理、超时或不可用时重新读取 WebView history，再安全交回 dispatcher。 */
    private fun performNativeFallback(callback: OnBackPressedCallback) {
        if (isFinishing || isDestroyed) return
        if (binding.webView.canGoBack()) {
            binding.webView.goBack()
            return
        }

        callback.isEnabled = false
        try {
            onBackPressedDispatcher.onBackPressed()
        } finally {
            if (!isFinishing && !isDestroyed) callback.isEnabled = true
        }
    }

    override fun onResume() {
        super.onResume()
        shellBridge?.onActivityResumed()
        viewportInsetsPublisher?.refresh()
        // 通知前端应用恢复，触发服务可用性补检；桥未安装时为 no-op。
        shellBridge?.notifyAppResumed()
    }

    override fun onPause() {
        shellBridge?.onActivityPaused()
        super.onPause()
    }

    override fun onStop() {
        shellBridge?.onActivityPaused()
        super.onStop()
    }

    override fun onDestroy() {
        mainHandler.removeCallbacks(splashTimeout)
        // 必须以 null 结算未完成的文件选择回调，避免 WebView 挂起或泄漏 ValueCallback。
        fileChooserSession.destroy()
        shellBridge?.destroy()
        shellBridge = null
        viewportInsetsPublisher?.dispose()
        viewportInsetsPublisher = null
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
