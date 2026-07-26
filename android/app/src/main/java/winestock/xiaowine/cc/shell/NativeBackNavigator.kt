package winestock.xiaowine.cc.shell

import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.OnBackPressedDispatcher

/**
 * 系统返回键：优先经 Shell Bridge 与前端协商，未处理时 WebView history / Activity finish。
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
