package winestock.xiaowine.cc

import android.Manifest
import android.content.Intent
import android.content.res.Configuration
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import winestock.xiaowine.cc.shell.MainShellCoordinator

/**
 * WineStock Android shell 的唯一 Activity 入口。
 *
 * 只负责系统生命周期回调与 [registerForActivityResult] 注册；
 * WebView / Bridge / 安全区 / 文件选择等组装见 [MainShellCoordinator] 与 web、shell 包。
 */
class MainActivity : ComponentActivity() {

    private lateinit var shell: MainShellCoordinator

    /**
     * 必须在 STARTED 前 register；结果交给 [MainShellCoordinator.onFileChooserResult]。
     */
    private val fileChooserLauncher: ActivityResultLauncher<Intent> =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            shell.onFileChooserResult(result)
        }

    /** WebView getUserMedia 摄像头的运行时权限请求；结果交给摄像头授权宿主结算。 */
    private val cameraPermissionLauncher: ActivityResultLauncher<String> =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            shell.onCameraPermissionResult(granted)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        // 必须在 super.onCreate 前接管系统启动窗口。
        val splashScreen = installSplashScreen()
        super.onCreate(savedInstanceState)
        shell =
            MainShellCoordinator(
                activity = this,
                launchFileChooser = { intent -> fileChooserLauncher.launch(intent) },
                requestCameraPermission = {
                    cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
                },
            )
        shell.start(splashScreen)
    }

    override fun onResume() {
        super.onResume()
        shell.onResume()
    }

    /** 系统 day/night 变化由当前 Activity 原地处理，避免销毁 WebView 和前端页面上下文。 */
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        if (::shell.isInitialized) {
            shell.onConfigurationChanged(newConfig)
        }
    }

    override fun onPause() {
        shell.onPause()
        super.onPause()
    }

    override fun onStop() {
        shell.onStop()
        super.onStop()
    }

    override fun onDestroy() {
        if (::shell.isInitialized) {
            shell.onDestroy()
        }
        super.onDestroy()
    }
}
