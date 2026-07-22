package winestock.xiaowine.cc.core

import winestock.xiaowine.cc.shell.EditableRuntimeConfig
import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

/** LocalCoreRuntimeManager 使用的窄 native 生命周期接口。 */
interface NativeCoreClient {
    fun initialize(): NativeCallResult<Unit>

    fun defaultRuntimeConfig(): NativeCallResult<EditableRuntimeConfig>

    fun validateRuntimeConfig(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeValidationResult>

    fun startLocalService(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeServiceState>

    fun stopLocalService(): NativeCallResult<NativeServiceState>

    fun restartLocalService(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeServiceState>

    fun getRuntimeState(): NativeCallResult<NativeServiceState>
}

/** System.loadLibrary + JNI JSON 的生产实现。 */
class JniNativeCoreClient(
    private val loader: NativeLibraryLoader = NativeLibraryLoader(),
) : NativeCoreClient {

    override fun initialize(): NativeCallResult<Unit> {
        loader.ensureLoaded()?.let { return NativeCallResult.Failure(it) }
        return call(NativeContract::parseInitialize) { NativeCoreBridge.nativeInitialize() }
    }

    override fun defaultRuntimeConfig(): NativeCallResult<EditableRuntimeConfig> =
        call(NativeContract::parseDefaultConfig) { NativeCoreBridge.nativeDefaultRuntimeConfig() }

    override fun validateRuntimeConfig(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeValidationResult> =
        call(NativeContract::parseValidation) {
            NativeCoreBridge.nativeValidateRuntimeConfig(NativeContract.requestJson(config, storage))
        }

    override fun startLocalService(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeServiceState> =
        call(NativeContract::parseServiceState) {
            NativeCoreBridge.nativeStartLocalService(NativeContract.requestJson(config, storage))
        }

    override fun stopLocalService(): NativeCallResult<NativeServiceState> =
        call(NativeContract::parseServiceState) { NativeCoreBridge.nativeStopLocalService() }

    override fun restartLocalService(
        config: EditableRuntimeConfig,
        storage: NativeStoragePaths,
    ): NativeCallResult<NativeServiceState> =
        call(NativeContract::parseServiceState) {
            NativeCoreBridge.nativeRestartLocalService(NativeContract.requestJson(config, storage))
        }

    override fun getRuntimeState(): NativeCallResult<NativeServiceState> =
        call(NativeContract::parseServiceState) { NativeCoreBridge.nativeGetRuntimeState() }

    private fun <T> call(
        parser: (String?) -> NativeCallResult<T>,
        invocation: () -> String?,
    ): NativeCallResult<T> {
        loader.ensureLoaded()?.let { return NativeCallResult.Failure(it) }
        return try {
            parser(invocation())
        } catch (_: LinkageError) {
            NativeCallResult.Failure(nativeUnavailable())
        } catch (_: RuntimeException) {
            NativeCallResult.Failure(nativeUnavailable())
        }
    }

    private fun nativeUnavailable() =
        ShellRuntimeError(
            ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE,
            "Android 本地服务组件调用失败，可切换为远端连接模式",
        )
}
