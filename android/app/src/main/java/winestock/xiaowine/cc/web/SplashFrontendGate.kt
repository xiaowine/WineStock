package winestock.xiaowine.cc.web

import androidx.core.splashscreen.SplashScreen

/**
 * 冷启动 SplashScreen 与前端首屏就绪门闩。
 *
 * 只由 Shell Bridge 的 frontendReady 握手解除；桥失败时由原生壳结束 WebView 会话。
 */
internal class SplashFrontendGate {
    private var frontendReady = false

    fun attach(splashScreen: SplashScreen) {
        splashScreen.setKeepOnScreenCondition { !frontendReady }
    }

    /** 标记前端就绪；幂等。 */
    fun markReady() {
        if (frontendReady) return
        frontendReady = true
    }

    fun cancelTimeout() = Unit
}
