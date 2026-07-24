package winestock.xiaowine.cc.web

import android.webkit.JavascriptInterface

/**
 * JS 侧薄接口：由 [SystemBarAppearanceController] 安装为 `WineStockSystemChrome`。
 * 不经 Shell Bridge 业务契约；Web/桌面无此对象时前端 no-op。
 */
internal class SystemBarAppearanceBridge(
    private val onDarkContentChanged: (darkContent: Boolean) -> Unit,
) {
    /**
     * @param enabled true：内容为深色遮罩，系统栏使用浅色图标；false：恢复浅色界面深色图标。
     */
    @JavascriptInterface
    fun setDarkContent(enabled: Boolean) {
        onDarkContentChanged(enabled)
    }
}
