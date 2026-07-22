package winestock.xiaowine.cc

import android.app.Application
import winestock.xiaowine.cc.core.LocalCoreRuntimeManager

/** Android 进程级 core 生命周期入口；Activity 重建不会替换或停止 manager。 */
class WineStockApplication : Application() {
    lateinit var localCoreRuntimeManager: LocalCoreRuntimeManager
        private set

    override fun onCreate() {
        super.onCreate()
        localCoreRuntimeManager = LocalCoreRuntimeManager.create(this)
    }
}
