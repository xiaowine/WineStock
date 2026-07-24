package winestock.xiaowine.cc

import android.content.Intent
import android.os.Bundle
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import winestock.xiaowine.cc.shell.MainShellCoordinator

/**
 * WineStock Android shell 的唯一 Activity 入口。
 *
 * 只负责系统生命周期回调与 [registerForActivityResult] 注册；
 * WebView / Bridge / 安全区 / 文件选择等组装见 [MainShellCoordinator] 与 web、shell 包。
 */
class MainActivity : AppCompatActivity() {

    private lateinit var shell: MainShellCoordinator

    /**
     * 必须在 STARTED 前 register；结果交给 [MainShellCoordinator.onFileChooserResult]。
     */
    private val fileChooserLauncher: ActivityResultLauncher<Intent> =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            shell.onFileChooserResult(result)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        // 必须在 super.onCreate 前接管系统启动窗口。
        val splashScreen = installSplashScreen()
        super.onCreate(savedInstanceState)
        shell =
            MainShellCoordinator(this) { intent ->
                fileChooserLauncher.launch(intent)
            }
        shell.start(splashScreen)
    }

    override fun onResume() {
        super.onResume()
        shell.onResume()
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
