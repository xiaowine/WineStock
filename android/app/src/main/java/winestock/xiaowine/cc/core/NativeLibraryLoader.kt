package winestock.xiaowine.cc.core

import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

/** 进程级、安全且可测试的 native library 加载器。 */
class NativeLibraryLoader(
    private val loadLibrary: (String) -> Unit = System::loadLibrary,
) {
    private sealed interface State {
        data object NotLoaded : State
        data object Loaded : State
        data class Failed(val error: ShellRuntimeError) : State
    }

    private var state: State = State.NotLoaded

    @Synchronized
    fun ensureLoaded(): ShellRuntimeError? =
        when (val current = state) {
            State.Loaded -> null
            is State.Failed -> current.error
            State.NotLoaded -> {
                try {
                    loadLibrary(LIBRARY_NAME)
                    state = State.Loaded
                    null
                } catch (_: LinkageError) {
                    fail()
                } catch (_: SecurityException) {
                    fail()
                }
            }
        }

    @Synchronized
    fun isLoaded(): Boolean = state == State.Loaded

    private fun fail(): ShellRuntimeError {
        val error =
            ShellRuntimeError(
                code = ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE,
                message = "Android 本地服务组件无法加载，可切换为远端连接模式",
            )
        state = State.Failed(error)
        return error
    }

    private companion object {
        const val LIBRARY_NAME = "winestock_android"
    }
}
