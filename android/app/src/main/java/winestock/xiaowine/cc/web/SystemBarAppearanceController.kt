package winestock.xiaowine.cc.web

import android.content.res.Configuration
import android.os.Build
import android.os.Handler
import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.core.content.ContextCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.core.view.insets.ColorProtection
import androidx.core.view.insets.ProtectionLayout
import winestock.xiaowine.cc.R

/**
 * 系统状态栏/导航栏外观控制器。
 *
 * 前端主题发布普通页面基线，图片查看等深色遮罩可临时覆盖；前端就绪前先跟随系统夜间模式。
 * 系统栏图标随内容明暗切换；透明导航栏下由 AndroidX 保护层提供稳定主题背景。不经 Shell Bridge 业务契约。
 */
internal class SystemBarAppearanceController(
    private val activity: ComponentActivity,
    private val contentView: () -> ProtectionLayout,
    private val mainHandler: Handler,
) {
    /** 当前前端内容是否要求浅色系统栏图标；用于 resume 后重放。 */
    private var darkContent = false
    private var hasFrontendAppearance = false
    private val navigationBarProtection = ColorProtection(WindowInsetsCompat.Side.BOTTOM)
    private var navigationBarProtectionInstalled = false

    fun applyDefaultBars() {
        applyDefaultBars(activity.resources.configuration)
    }

    /**
     * 前端接管前跟随新的系统模式；接管后保留前端基线或临时覆盖，等待 media query 发布新值。
     * 这样系统切换不会短暂覆盖与系统相反的手动主题。
     */
    fun onConfigurationChanged(newConfig: Configuration) {
        if (hasFrontendAppearance) {
            reapply()
        } else {
            applyDefaultBars(newConfig)
        }
    }

    private fun applyDefaultBars(configuration: Configuration) {
        darkContent =
            configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                Configuration.UI_MODE_NIGHT_YES
        apply(lightBars = !darkContent)
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
                    hasFrontendAppearance = true
                    darkContent = enabled
                    apply(lightBars = !enabled)
                }
            },
            JS_INTERFACE_NAME,
        )
    }

    /**
     * @param lightBars true：浅色栏区 + 深色图标；
     * false：深色内容上的浅色图标。导航栏保护色跟随内容主题，避免底部系统栏与页面混在一起。
     */
    private fun apply(lightBars: Boolean) {
        val root = runCatching { contentView() }.getOrNull() ?: return
        WindowInsetsControllerCompat(activity.window, root).apply {
            isAppearanceLightStatusBars = lightBars
            isAppearanceLightNavigationBars = lightBars
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val protectionColor =
                if (lightBars) {
                    R.color.navigation_bar_protection_light
                } else {
                    R.color.navigation_bar_protection_dark
                }
            navigationBarProtection.setColor(ContextCompat.getColor(activity, protectionColor))
            if (!navigationBarProtectionInstalled) {
                root.setProtections(listOf(navigationBarProtection))
                navigationBarProtectionInstalled = true
            }
            // 应用保护层拥有最终背景，关闭系统自动 scrim，避免重复叠色和厂商差异。
            activity.window.isNavigationBarContrastEnforced = false
        }
    }

    companion object {
        const val JS_INTERFACE_NAME = "WineStockSystemChrome"
    }
}
