package winestock.xiaowine.cc.web

import android.os.Build
import android.os.Handler
import android.view.View
import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.core.view.WindowInsetsControllerCompat

/**
 * 系统状态栏/导航栏外观控制器。
 *
 * 默认浅色业务 UI 使用深色图标；图片查看等深色遮罩经 [SystemBarAppearanceBridge]
 * 切到浅色图标并关闭导航栏 contrast 强制。不经 Shell Bridge 业务契约。
 */
internal class SystemBarAppearanceController(
    private val activity: ComponentActivity,
    private val contentView: () -> View,
    private val mainHandler: Handler,
) {
    /** 图片查看等场景是否要求深色内容系统栏（浅色图标）；用于 resume 后重放。 */
    private var darkContent = false

    fun applyDefaultLightBars() {
        darkContent = false
        apply(lightBars = true)
    }

    /** 从文件选择器等返回后按当前缓存状态重放外观。 */
    fun reapply() {
        apply(lightBars = !darkContent)
    }

    fun installJavascriptInterface(webView: WebView) {
        webView.addJavascriptInterface(
            SystemBarAppearanceBridge { enabled ->
                mainHandler.post {
                    if (activity.isFinishing || activity.isDestroyed) return@post
                    darkContent = enabled
                    apply(lightBars = !enabled)
                }
            },
            JS_INTERFACE_NAME,
        )
    }

    /**
     * @param lightBars true：浅色栏区 + 深色图标（默认业务 UI）；
     * false：深色内容上的浅色图标，并关闭导航栏对比度强制以免系统再铺浅色底。
     */
    private fun apply(lightBars: Boolean) {
        val root = runCatching { contentView() }.getOrNull() ?: return
        WindowInsetsControllerCompat(activity.window, root).apply {
            isAppearanceLightStatusBars = lightBars
            isAppearanceLightNavigationBars = lightBars
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            activity.window.isNavigationBarContrastEnforced = lightBars
        }
    }

    companion object {
        const val JS_INTERFACE_NAME = "WineStockSystemChrome"
    }
}
