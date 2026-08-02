package winestock.xiaowine.cc

import android.Manifest
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.res.Configuration
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.splashscreen.SplashScreen
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.Lifecycle
import winestock.xiaowine.cc.shell.MainShellCoordinator
import winestock.xiaowine.cc.web.WebViewCompatibility
import winestock.xiaowine.cc.web.WebViewCompatibilityResult
import winestock.xiaowine.cc.web.WebViewCompatibilityScreen
import winestock.xiaowine.cc.web.WebViewIncompatibilityReason

/**
 * WineStock Android shell 的唯一 Activity 入口。
 *
 * Manifest 将 Activity 锁定为 sensorPortrait，禁止进入横屏；本类负责 WebView 启动门禁、
 * 系统生命周期回调与 [registerForActivityResult] 注册；
 * WebView / Bridge / 安全区 / 文件选择等组装见 [MainShellCoordinator] 与 web、shell 包。
 */
class MainActivity : ComponentActivity() {

    private var shell: MainShellCoordinator? = null
    private var compatibilityScreen: WebViewCompatibilityScreen? = null

    /**
     * 必须在 STARTED 前 register；结果交给 [MainShellCoordinator.onFileChooserResult]。
     */
    private val fileChooserLauncher: ActivityResultLauncher<Intent> =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            shell?.onFileChooserResult(result)
        }

    /** WebView getUserMedia 摄像头的运行时权限请求；结果交给摄像头授权宿主结算。 */
    private val cameraPermissionLauncher: ActivityResultLauncher<String> =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            shell?.onCameraPermissionResult(granted)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        // 必须在 super.onCreate 前接管系统启动窗口。
        val splashScreen = installSplashScreen()
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        when (val compatibility = checkedCompatibility()) {
            is WebViewCompatibilityResult.Supported -> startShell(splashScreen)
            is WebViewCompatibilityResult.Unsupported -> showCompatibilityScreen(compatibility)
        }
    }

    private fun startShell(splashScreen: SplashScreen? = null) {
        compatibilityScreen?.destroy()
        compatibilityScreen = null
        window.setBackgroundDrawableResource(R.color.web_background)
        val coordinator = MainShellCoordinator(
            activity = this,
            launchFileChooser = { intent -> fileChooserLauncher.launch(intent) },
            requestCameraPermission = {
                cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
            },
            onBridgeFailure = ::handleShellBridgeFailure,
        )
        shell = coordinator
        coordinator.start(splashScreen)
        if (lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) {
            coordinator.onResume()
        }
    }

    private fun handleShellBridgeFailure(_message: String) {
        if (shell == null) return
        shell = null
        showCompatibilityScreen(
            WebViewCompatibilityResult.Unsupported(
                provider = runCatching { android.webkit.WebView.getCurrentWebViewPackage() }
                    .getOrNull()
                    ?.let { packageInfo ->
                        winestock.xiaowine.cc.web.WebViewProviderSnapshot(
                            packageName = packageInfo.packageName,
                            versionName = packageInfo.versionName,
                        )
                    },
                reason = WebViewIncompatibilityReason.SHELL_BRIDGE_UNAVAILABLE,
            ),
        )
    }

    private fun showCompatibilityScreen(result: WebViewCompatibilityResult.Unsupported) {
        compatibilityScreen =
            WebViewCompatibilityScreen(
                activity = this,
                recheck = WebViewCompatibility::check,
                onSupported = { startShell() },
            ).also { it.show(result) }
    }

    /** Debug APK 可由 adb 强制展示阻断页，以便不降级系统 provider 也能验收恢复 UI。 */
    private fun checkedCompatibility(): WebViewCompatibilityResult {
        val result = WebViewCompatibility.check()
        val isDebuggable = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        if (!isDebuggable || !intent.getBooleanExtra(EXTRA_FORCE_WEBVIEW_BLOCK, false)) {
            return result
        }
        return WebViewCompatibilityResult.Unsupported(
            provider = result.provider,
            reason = WebViewIncompatibilityReason.VERSION_TOO_OLD,
        )
    }

    override fun onResume() {
        super.onResume()
        shell?.onResume()
    }

    /** 系统 day/night 变化由当前 Activity 原地处理，避免销毁 WebView 和前端页面上下文。 */
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        compatibilityScreen?.onConfigurationChanged(newConfig)
        shell?.onConfigurationChanged(newConfig)
    }

    override fun onPause() {
        shell?.onPause()
        super.onPause()
    }

    override fun onStop() {
        shell?.onStop()
        super.onStop()
    }

    override fun onDestroy() {
        compatibilityScreen?.destroy()
        shell?.onDestroy()
        super.onDestroy()
    }

    companion object {
        private const val EXTRA_FORCE_WEBVIEW_BLOCK =
            "winestock.xiaowine.cc.extra.FORCE_WEBVIEW_BLOCK"
    }
}
