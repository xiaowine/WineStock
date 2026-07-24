package winestock.xiaowine.cc.web

import android.os.Handler
import androidx.core.splashscreen.SplashScreen

/**
 * 冷启动 SplashScreen 与前端首屏就绪门闩。
 *
 * 前端 ready（Shell Bridge）或 onPageFinished 兜底、或超时后放行；[markReady] 幂等。
 */
internal class SplashFrontendGate(
    private val mainHandler: Handler,
    private val timeoutMs: Long,
) {
    private var frontendReady = false
    private val splashTimeout = Runnable { markReady() }

    fun attach(splashScreen: SplashScreen) {
        splashScreen.setKeepOnScreenCondition { !frontendReady }
        mainHandler.postDelayed(splashTimeout, timeoutMs)
    }

    /** 标记前端就绪并清除兜底超时；幂等。 */
    fun markReady() {
        if (frontendReady) return
        frontendReady = true
        mainHandler.removeCallbacks(splashTimeout)
    }

    fun cancelTimeout() {
        mainHandler.removeCallbacks(splashTimeout)
    }
}
