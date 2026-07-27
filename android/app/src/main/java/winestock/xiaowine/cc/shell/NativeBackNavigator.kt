package winestock.xiaowine.cc.shell

import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.OnBackPressedDispatcher
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

/**
 * 系统返回键：输入法可见时先隐藏并消费；否则经前端协商，未处理时回退 WebView / Activity。
 */
internal class NativeBackNavigator(
    private val activity: ComponentActivity,
    private val webView: () -> WebView,
    private val shellBridge: () -> ShellBridgeHost?,
) {
    fun install(dispatcher: OnBackPressedDispatcher) {
        dispatcher.addCallback(
            activity,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (hideImeIfVisible()) return

                    when (
                        shellBridge()?.requestNativeBack(webView().canGoBack()) { handled ->
                            if (!handled) performNativeFallback(this)
                        }
                    ) {
                        ShellBridgeHost.NativeBackDispatchResult.DISPATCHED,
                        ShellBridgeHost.NativeBackDispatchResult.ALREADY_PENDING,
                        -> return
                        ShellBridgeHost.NativeBackDispatchResult.UNAVAILABLE,
                        null,
                        -> performNativeFallback(this)
                    }
                }
            },
        )
    }

    /** 输入法是系统级临时层；本次返回只关闭 IME，不进入前端浮层或路由协商。 */
    private fun hideImeIfVisible(): Boolean {
        val target = webView()
        val insets = ViewCompat.getRootWindowInsets(target) ?: return false
        if (!insets.isVisible(WindowInsetsCompat.Type.ime())) return false

        WindowCompat.getInsetsController(activity.window, target)
            .hide(WindowInsetsCompat.Type.ime())
        return true
    }

    /** 协商未处理、超时或不可用时重新读取 WebView history，再安全交回 dispatcher。 */
    private fun performNativeFallback(callback: OnBackPressedCallback) {
        if (activity.isFinishing || activity.isDestroyed) return
        if (webView().canGoBack()) {
            webView().goBack()
            return
        }

        callback.isEnabled = false
        try {
            activity.onBackPressedDispatcher.onBackPressed()
        } finally {
            if (!activity.isFinishing && !activity.isDestroyed) {
                callback.isEnabled = true
            }
        }
    }
}
