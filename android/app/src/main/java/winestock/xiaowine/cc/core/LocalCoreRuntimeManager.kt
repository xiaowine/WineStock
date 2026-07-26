package winestock.xiaowine.cc.core

import android.content.Context
import java.net.URI
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import winestock.xiaowine.cc.shell.AndroidRuntimeSnapshot
import winestock.xiaowine.cc.shell.DEFAULT_RUNTIME_CONFIG
import winestock.xiaowine.cc.shell.EditableRuntimeConfig
import winestock.xiaowine.cc.shell.RemoteRuntimeConfigFallbackValidator
import winestock.xiaowine.cc.shell.RuntimeConfigFields
import winestock.xiaowine.cc.shell.RuntimeConfigRepository
import winestock.xiaowine.cc.shell.RuntimeConfigStore
import winestock.xiaowine.cc.shell.RuntimeConfigValidationResult
import winestock.xiaowine.cc.shell.RuntimeModes
import winestock.xiaowine.cc.shell.RuntimeServiceSnapshot
import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

data class ApplyRuntimeConfigResult(
    val validation: RuntimeConfigValidationResult,
    val applied: Boolean,
    val snapshot: AndroidRuntimeSnapshot,
    val error: ShellRuntimeError? = null,
)

/**
 * Application 进程级本地 core 所有者。
 *
 * 所有 JNI、配置事务和数据库启停都在同一后台 executor 串行执行；Activity 只订阅快照，
 * 旋转、页面 reload 或短暂后台不会停止服务。
 */
class LocalCoreRuntimeManager(
    private val nativeClient: NativeCoreClient,
    private val configRepository: RuntimeConfigRepository,
    private val storageProvider: () -> NativeCallResult<NativeStoragePaths>,
    private val executor: ExecutorService =
        Executors.newSingleThreadExecutor { runnable ->
            Thread(runnable, "winestock-core-manager").apply { isDaemon = true }
        },
    private val restartScheduler: ScheduledExecutorService =
        Executors.newSingleThreadScheduledExecutor { runnable ->
            Thread(runnable, "winestock-core-restart").apply { isDaemon = true }
        },
    /** 意外失败后自动重启的退避序列；长度即最大自动尝试次数。 */
    private val autoRestartDelaysMs: LongArray = longArrayOf(1_000, 3_000),
) {
    private val listeners = CopyOnWriteArraySet<(AndroidRuntimeSnapshot) -> Unit>()

    @Volatile
    private var snapshot =
        AndroidRuntimeSnapshot(
            configStatus = "unconfigured",
            config = DEFAULT_RUNTIME_CONFIG,
            initialized = false,
            service = RuntimeServiceSnapshot("local", "stopped"),
            nativeAvailable = false,
        )

    private var nativeReady = false
    private var storagePaths: NativeStoragePaths? = null
    private var sharedDefaultConfig = DEFAULT_RUNTIME_CONFIG

    // 自动重启状态只在 manager executor 线程上读写；手动操作会推进 epoch 作废挂起的尝试。
    private var autoRestartAttempts = 0
    private var autoRestartEpoch = 0

    init {
        submit { initializeInternal() }
    }

    fun addListener(listener: (AndroidRuntimeSnapshot) -> Unit): AutoCloseable {
        listeners += listener
        return AutoCloseable { listeners -= listener }
    }

    fun getRuntimeSnapshot(): CompletableFuture<AndroidRuntimeSnapshot> =
        submit {
            refreshNativeStateIfNeeded()
            snapshot
        }

    fun validateRuntimeConfig(
        config: EditableRuntimeConfig,
    ): CompletableFuture<RuntimeConfigValidationResult> =
        submit { authorityValidate(config).validation }

    fun applyRuntimeConfig(
        config: EditableRuntimeConfig,
    ): CompletableFuture<ApplyRuntimeConfigResult> =
        submit { applyInternal(config) }

    fun startLocalService(): CompletableFuture<AndroidRuntimeSnapshot> =
        submit { startCurrentInternal() }

    fun stopLocalService(): CompletableFuture<AndroidRuntimeSnapshot> =
        submit { stopCurrentInternal() }

    fun restartLocalService(): CompletableFuture<AndroidRuntimeSnapshot> =
        submit { restartCurrentInternal() }

    /** 仅供 JVM 单元测试清理后台线程；生产进程生命周期不由 Activity 关闭。 */
    fun shutdownForTests() {
        executor.shutdownNow()
        restartScheduler.shutdownNow()
    }

    private fun initializeInternal(): AndroidRuntimeSnapshot {
        val loaded = safeLoadConfig()
        when (val initialized = nativeClient.initialize()) {
            is NativeCallResult.Success -> nativeReady = true
            is NativeCallResult.Failure -> {
                nativeReady = false
                return publish(initializeWithoutLocalCore(loaded, initialized.error))
            }
        }

        when (val defaultResult = nativeClient.defaultRuntimeConfig()) {
            is NativeCallResult.Success -> sharedDefaultConfig = defaultResult.value
            is NativeCallResult.Failure -> {
                handleNativeAvailabilityFailure(defaultResult.error)
                return publish(
                    if (loaded is RuntimeConfigRepository.Loaded.Present) {
                        initializeWithoutLocalCore(loaded, defaultResult.error)
                    } else {
                        unconfiguredSnapshot(
                            config = DEFAULT_RUNTIME_CONFIG,
                            error = defaultResult.error,
                        )
                    },
                )
            }
        }

        when (val storage = storageProvider()) {
            is NativeCallResult.Success -> storagePaths = storage.value
            is NativeCallResult.Failure -> {
                storagePaths = null
                return publish(initializeWithoutLocalCore(loaded, storage.error))
            }
        }

        return when (loaded) {
            // 缺少持久配置时只发布默认草稿，不启动本地 core；首次启动由前端 apply 触发。
            RuntimeConfigRepository.Loaded.Missing -> publish(unconfiguredSnapshot(sharedDefaultConfig))
            RuntimeConfigRepository.Loaded.Invalid ->
                publish(
                    invalidSnapshot(
                        sharedDefaultConfig,
                        ShellRuntimeError(
                            ShellErrorCodes.CONFIG_INVALID,
                            "已保存的运行配置无法解析，请重新应用默认配置",
                        ),
                    ),
                )
            is RuntimeConfigRepository.Loaded.Present -> activatePersisted(loaded.config)
        }
    }

    private fun initializeWithoutLocalCore(
        loaded: RuntimeConfigRepository.Loaded,
        error: ShellRuntimeError,
    ): AndroidRuntimeSnapshot =
        when (loaded) {
            RuntimeConfigRepository.Loaded.Missing ->
                unconfiguredSnapshot(DEFAULT_RUNTIME_CONFIG, error)
            RuntimeConfigRepository.Loaded.Invalid ->
                invalidSnapshot(DEFAULT_RUNTIME_CONFIG, error)
            is RuntimeConfigRepository.Loaded.Present -> {
                val fallback = RemoteRuntimeConfigFallbackValidator.validate(loaded.config)
                if (fallback.valid) {
                    remoteSnapshot(
                        config = fallback.normalizedConfig ?: loaded.config,
                        nativeAvailable = false,
                    )
                } else if (RuntimeModes.isRemote(loaded.config.mode)) {
                    // native 不可用时仍能识别远端配置；地址本身无效则回到可修复的未初始化态。
                    invalidSnapshot(loaded.config, configInvalid())
                } else {
                    AndroidRuntimeSnapshot(
                        configStatus = "configured",
                        config = loaded.config,
                        initialized = true,
                        service = RuntimeServiceSnapshot("local", "failed", error = error),
                        nativeAvailable = false,
                    )
                }
            }
        }

    private fun activatePersisted(config: EditableRuntimeConfig): AndroidRuntimeSnapshot {
        val attempt = authorityValidate(config)
        if (!attempt.validation.valid) {
            // 持久配置字段本身无效才需要重新初始化；native/JNI 或存储故障仍表示配置已确认，
            // 让前端进入服务失败/恢复路径，而不是误显示首次设置漏斗。
            if (attempt.error != null && !isPersistedConfigInvalid(attempt.error)) {
                if (RuntimeModes.isRemote(config.mode)) {
                    val fallback = RemoteRuntimeConfigFallbackValidator.validate(config)
                    if (fallback.valid) {
                        return publish(
                            remoteSnapshot(
                                config = fallback.normalizedConfig ?: config,
                                nativeAvailable = localAvailable(),
                            ),
                        )
                    }
                    return publish(
                        remoteSnapshot(
                            config = config,
                            nativeAvailable = localAvailable(),
                            phase = "failed",
                            error = attempt.error,
                        ),
                    )
                }
                return publish(
                    localSnapshot(
                        configStatus = "configured",
                        config = config,
                        phase = "failed",
                        initialized = true,
                        error = attempt.error,
                    ),
                )
            }
            return publish(invalidSnapshot(config, attempt.error ?: configInvalid()))
        }
        val normalized = attempt.validation.normalizedConfig ?: config
        if (RuntimeModes.isRemote(normalized.mode)) {
            if (normalized != config && !safeSaveConfig(normalized)) {
                return publish(
                    remoteSnapshot(
                        config = config,
                        nativeAvailable = localAvailable(),
                        error = configUnavailable(),
                    ),
                )
            }
            return publish(
                remoteSnapshot(
                    config = normalized,
                    nativeAvailable = localAvailable(),
                ),
            )
        }

        publish(localSnapshot("configured", normalized, "starting"))
        return when (val started = startLocalNative(normalized)) {
            is NativeCallResult.Success -> {
                val effectiveConfig = effectiveLocalConfig(normalized, started.value)
                if (effectiveConfig != normalized && !safeSaveConfig(effectiveConfig)) {
                    nativeClient.stopLocalService()
                    publish(localSnapshot("configured", normalized, "failed", error = configUnavailable()))
                } else {
                    publish(localRunningSnapshot(effectiveConfig, started.value))
                }
            }
            is NativeCallResult.Failure ->
                publish(localSnapshot("configured", normalized, "failed", error = started.error))
        }
    }

    private fun applyInternal(config: EditableRuntimeConfig): ApplyRuntimeConfigResult {
        cancelPendingAutoRestart()
        val attempt = authorityValidate(config)
        if (!attempt.validation.valid) {
            return ApplyRuntimeConfigResult(
                validation = attempt.validation,
                applied = false,
                snapshot = snapshot,
                error = attempt.error,
            )
        }
        val previous = snapshot
        val normalizedCandidate = attempt.validation.normalizedConfig ?: config
        // UI 不暴露 self-hosted 端口；首次确认后才用端口 0 请求系统分配并持久化实际端口。
        val candidate =
            if (!previous.initialized && normalizedCandidate.mode == RuntimeModes.SELF_HOSTED) {
                normalizedCandidate.copy(port = 0)
            } else {
                normalizedCandidate
            }

        if (RuntimeModes.isRemote(candidate.mode)) {
            if (previous.service.ownership == "local" && previous.service.phase == "running") {
                publish(previous.copy(service = previous.service.copy(phase = "stopping")))
                when (val stopped = nativeClient.stopLocalService()) {
                    is NativeCallResult.Failure -> {
                        val failed = previous.withError(stopped.error)
                        publish(failed)
                        return ApplyRuntimeConfigResult(attempt.validation, false, failed, stopped.error)
                    }
                    is NativeCallResult.Success -> Unit
                }
            }
            if (!safeSaveConfig(candidate)) {
                val restored = restorePrevious(previous, configUnavailable())
                return ApplyRuntimeConfigResult(
                    attempt.validation,
                    false,
                    restored,
                    configUnavailable(),
                )
            }
            val applied = remoteSnapshot(candidate, nativeAvailable = localAvailable())
            publish(applied)
            return ApplyRuntimeConfigResult(attempt.validation, true, applied)
        }

        if (!localAvailable()) {
            val error = nativeUnavailable()
            val failed = previous.withError(error)
            publish(failed)
            return ApplyRuntimeConfigResult(attempt.validation, false, failed, error)
        }

        if (previous.service.ownership == "local" && previous.service.phase == "running") {
            publish(previous.copy(service = previous.service.copy(phase = "stopping")))
            when (val stopped = nativeClient.stopLocalService()) {
                is NativeCallResult.Failure -> {
                    val failed = previous.withError(stopped.error)
                    publish(failed)
                    return ApplyRuntimeConfigResult(attempt.validation, false, failed, stopped.error)
                }
                is NativeCallResult.Success -> Unit
            }
        }

        // 首次应用尚未完成持久化前保持 initialized=false；已有配置替换时沿用 true。
        publish(localSnapshot("configured", candidate, "starting", initialized = previous.initialized))
        return when (val started = startLocalNative(candidate)) {
            is NativeCallResult.Failure -> {
                val restored = restorePrevious(previous, started.error)
                ApplyRuntimeConfigResult(attempt.validation, false, restored, started.error)
            }
            is NativeCallResult.Success -> {
                val effectiveConfig = effectiveLocalConfig(candidate, started.value)
                if (!safeSaveConfig(effectiveConfig)) {
                    nativeClient.stopLocalService()
                    val error = configUnavailable()
                    val restored = restorePrevious(previous, error)
                    ApplyRuntimeConfigResult(attempt.validation, false, restored, error)
                } else {
                    val applied = localRunningSnapshot(effectiveConfig, started.value)
                    publish(applied)
                    ApplyRuntimeConfigResult(attempt.validation, true, applied)
                }
            }
        }
    }

    private fun startCurrentInternal(): AndroidRuntimeSnapshot {
        cancelPendingAutoRestart()
        val current = snapshot
        if (current.configStatus != "configured" || current.service.ownership != "local") {
            return publish(current.withError(unsupportedMode()))
        }
        if (!localAvailable()) return publish(current.withError(nativeUnavailable()))
        publish(current.copy(service = current.service.copy(phase = "starting", error = null)))
        return when (val started = startLocalNative(current.config)) {
            is NativeCallResult.Success -> {
                val effectiveConfig = effectiveLocalConfig(current.config, started.value)
                if (effectiveConfig != current.config && !safeSaveConfig(effectiveConfig)) {
                    nativeClient.stopLocalService()
                    publish(
                        localSnapshot(
                            "configured",
                            current.config,
                            "failed",
                            error = configUnavailable(),
                        ),
                    )
                } else {
                    publish(localRunningSnapshot(effectiveConfig, started.value))
                }
            }
            is NativeCallResult.Failure ->
                publish(localSnapshot("configured", current.config, "failed", error = started.error))
        }
    }

    private fun stopCurrentInternal(): AndroidRuntimeSnapshot {
        cancelPendingAutoRestart()
        val current = snapshot
        if (current.service.ownership != "local") {
            return publish(current.withError(unsupportedMode()))
        }
        if (!localAvailable()) return publish(current.withError(nativeUnavailable()))
        publish(current.copy(service = current.service.copy(phase = "stopping", error = null)))
        return when (val stopped = nativeClient.stopLocalService()) {
            is NativeCallResult.Success ->
                publish(localSnapshot(current.configStatus, current.config, "stopped"))
            is NativeCallResult.Failure ->
                publish(localSnapshot(current.configStatus, current.config, "failed", error = stopped.error))
        }
    }

    private fun restartCurrentInternal(): AndroidRuntimeSnapshot {
        cancelPendingAutoRestart()
        val current = snapshot
        if (current.configStatus != "configured" || current.service.ownership != "local") {
            return publish(current.withError(unsupportedMode()))
        }
        val paths = storagePaths ?: return publish(current.withError(nativeUnavailable()))
        return performLocalRestart(current, paths)
    }

    /** 手动与自动重启共用的恢复流程；调用前提是本地 configured 快照。 */
    private fun performLocalRestart(
        current: AndroidRuntimeSnapshot,
        paths: NativeStoragePaths,
    ): AndroidRuntimeSnapshot {
        publish(current.copy(service = current.service.copy(phase = "starting", error = null)))
        return when (val restarted = restartLocalNative(current.config, paths)) {
            is NativeCallResult.Success -> {
                val effectiveConfig = effectiveLocalConfig(current.config, restarted.value)
                if (effectiveConfig != current.config && !safeSaveConfig(effectiveConfig)) {
                    nativeClient.stopLocalService()
                    publish(
                        localSnapshot(
                            "configured",
                            current.config,
                            "failed",
                            error = configUnavailable(),
                        ),
                    )
                } else {
                    publish(localRunningSnapshot(effectiveConfig, restarted.value))
                }
            }
            is NativeCallResult.Failure ->
                publish(localSnapshot("configured", current.config, "failed", error = restarted.error))
        }
    }

    /**
     * 意外失败（此前 running 的服务变为 failed，或自动恢复链上的再次失败）时按退避
     * 自动重启，最多 autoRestartDelaysMs.size 次；手动操作与成功 running 都会复位。
     */
    private fun maybeScheduleAutoRestart(
        previous: AndroidRuntimeSnapshot,
        next: AndroidRuntimeSnapshot,
    ) {
        if (next.service.ownership != "local" || next.service.phase != "failed") return
        if (next.configStatus != "configured" || !next.initialized || !localAvailable()) return
        val unexpectedDeath =
            previous.service.ownership == "local" && previous.service.phase == "running"
        val recoveryChain = autoRestartAttempts > 0
        if (!unexpectedDeath && !recoveryChain) return
        if (autoRestartAttempts >= autoRestartDelaysMs.size) return
        val delayMs = autoRestartDelaysMs[autoRestartAttempts]
        autoRestartAttempts += 1
        val epoch = autoRestartEpoch
        restartScheduler.schedule(
            { submit { autoRestartInternal(epoch) } },
            delayMs,
            TimeUnit.MILLISECONDS,
        )
    }

    private fun autoRestartInternal(epoch: Int): AndroidRuntimeSnapshot {
        val current = snapshot
        if (epoch != autoRestartEpoch) return current
        if (
            current.configStatus != "configured" ||
            current.service.ownership != "local" ||
            current.service.phase != "failed"
        ) {
            return current
        }
        val paths = storagePaths ?: return current
        return performLocalRestart(current, paths)
    }

    private fun cancelPendingAutoRestart() {
        autoRestartEpoch += 1
        autoRestartAttempts = 0
    }

    private fun refreshNativeStateIfNeeded() {
        val current = snapshot
        if (!localAvailable() || current.service.ownership != "local" || current.service.phase != "running") {
            return
        }
        when (val state = nativeClient.getRuntimeState()) {
            is NativeCallResult.Success -> {
                if (state.value.phase != "running" || state.value.error != null) {
                    // 用户停止只经 stopLocalService 产生；此处观察到的非 running 一律按异常退出
                    // 映射为 failed，保证前端进入恢复语义而不是"已停止"。
                    val unexpectedPhase = state.value.phase
                    val phase =
                        if (unexpectedPhase == "stopped" || unexpectedPhase == "failed") "failed"
                        else unexpectedPhase
                    val error = state.value.error ?: serviceCrashed()
                    publish(
                        localSnapshot(
                            current.configStatus,
                            current.config,
                            phase,
                            state = state.value,
                            error = error,
                        ),
                    )
                }
            }
            is NativeCallResult.Failure -> publish(current.withError(state.error))
        }
    }

    private fun restorePrevious(
        previous: AndroidRuntimeSnapshot,
        operationError: ShellRuntimeError,
    ): AndroidRuntimeSnapshot {
        var restored = previous
        if (
            previous.service.ownership == "local" &&
            previous.service.phase == "running" &&
            localAvailable()
        ) {
            restored =
                when (val restarted = startLocalNative(previous.config)) {
                    is NativeCallResult.Success -> {
                        val effectiveConfig = effectiveLocalConfig(previous.config, restarted.value)
                        if (effectiveConfig != previous.config && !safeSaveConfig(effectiveConfig)) {
                            nativeClient.stopLocalService()
                            localSnapshot(
                                "configured",
                                previous.config,
                                "failed",
                                error = configUnavailable(),
                            )
                        } else {
                            localRunningSnapshot(
                                effectiveConfig,
                                restarted.value,
                                initialized = previous.initialized,
                            )
                        }
                    }
                    is NativeCallResult.Failure ->
                        localSnapshot("configured", previous.config, "failed", error = restarted.error)
                }
        }
        restored = restored.withError(operationError)
        return publish(restored)
    }

    private fun authorityValidate(config: EditableRuntimeConfig): ValidationAttempt {
        val paths = storagePaths
        if (!nativeReady || paths == null) {
            val validation = RemoteRuntimeConfigFallbackValidator.validate(config)
            return ValidationAttempt(
                validation,
                if (validation.valid) null else nativeUnavailable(),
            )
        }
        return when (val result = nativeClient.validateRuntimeConfig(config, paths)) {
            is NativeCallResult.Success ->
                ValidationAttempt(
                    RuntimeConfigValidationResult(
                        fieldErrors = result.value.fieldErrors,
                        normalizedConfig = result.value.normalizedConfig,
                    ),
                    null,
                )
            is NativeCallResult.Failure -> {
                handleNativeAvailabilityFailure(result.error)
                // 远端模式不依赖本地 native/storage；组件故障时仍允许保存并连接远端。
                if (RuntimeModes.isRemote(config.mode)) {
                    val fallback = RemoteRuntimeConfigFallbackValidator.validate(config)
                    if (fallback.valid) {
                        return ValidationAttempt(fallback, null)
                    }
                }
                ValidationAttempt(validationForError(result.error), result.error)
            }
        }
    }

    private fun startNative(config: EditableRuntimeConfig): NativeCallResult<NativeServiceState> {
        val paths = storagePaths ?: return NativeCallResult.Failure(nativeUnavailable())
        val result = nativeClient.startLocalService(config, paths)
        if (result is NativeCallResult.Failure) handleNativeAvailabilityFailure(result.error)
        return result
    }

    private fun startLocalNative(config: EditableRuntimeConfig): NativeCallResult<NativeServiceState> {
        val first = startNative(config)
        if (
            first is NativeCallResult.Failure &&
            first.error.code == ShellErrorCodes.PORT_IN_USE &&
            config.mode == RuntimeModes.SELF_HOSTED &&
            config.port != 0
        ) {
            return startNative(config.copy(port = 0))
        }
        return first
    }

    private fun restartLocalNative(
        config: EditableRuntimeConfig,
        paths: NativeStoragePaths,
    ): NativeCallResult<NativeServiceState> {
        val first = nativeClient.restartLocalService(config, paths)
        if (first is NativeCallResult.Failure) handleNativeAvailabilityFailure(first.error)
        if (
            first is NativeCallResult.Failure &&
            first.error.code == ShellErrorCodes.PORT_IN_USE &&
            config.mode == RuntimeModes.SELF_HOSTED &&
            config.port != 0
        ) {
            val retry = nativeClient.restartLocalService(config.copy(port = 0), paths)
            if (retry is NativeCallResult.Failure) handleNativeAvailabilityFailure(retry.error)
            return retry
        }
        return first
    }

    private fun effectiveLocalConfig(
        config: EditableRuntimeConfig,
        state: NativeServiceState,
    ): EditableRuntimeConfig {
        if (config.mode != RuntimeModes.SELF_HOSTED || state.phase != "running") return config
        val actualPort =
            state.apiBaseUrl
                ?.let { runCatching { URI(it).port }.getOrDefault(-1) }
                ?: -1
        return if (actualPort in 1..65535 && actualPort != config.port) {
            config.copy(port = actualPort)
        } else {
            config
        }
    }

    private fun handleNativeAvailabilityFailure(error: ShellRuntimeError) {
        if (error.code == ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE) {
            nativeReady = false
        }
    }

    private fun validationForError(error: ShellRuntimeError): RuntimeConfigValidationResult {
        val field = error.field ?: RuntimeConfigFields.MODE
        return RuntimeConfigValidationResult(mapOf(field to listOf(error.message)))
    }

    private fun localRunningSnapshot(
        config: EditableRuntimeConfig,
        state: NativeServiceState,
        initialized: Boolean = true,
    ): AndroidRuntimeSnapshot =
        localSnapshot(
            configStatus = "configured",
            config = config,
            phase = state.phase,
            initialized = initialized,
            state = state,
            error = state.error,
        )

    private fun localSnapshot(
        configStatus: String,
        config: EditableRuntimeConfig,
        phase: String,
        initialized: Boolean = configStatus == "configured",
        state: NativeServiceState? = null,
        error: ShellRuntimeError? = null,
    ) =
        AndroidRuntimeSnapshot(
            configStatus = configStatus,
            config = config,
            initialized = initialized,
            service =
                RuntimeServiceSnapshot(
                    ownership = "local",
                    phase = phase,
                    apiBaseUrl = state?.apiBaseUrl,
                    boundAddress = state?.boundAddress,
                    error = error,
                ),
            nativeAvailable = localAvailable(),
        )

    private fun remoteSnapshot(
        config: EditableRuntimeConfig,
        nativeAvailable: Boolean,
        phase: String = "running",
        error: ShellRuntimeError? = null,
    ) =
        AndroidRuntimeSnapshot(
            configStatus = "configured",
            config = config,
            initialized = true,
            service =
                RuntimeServiceSnapshot(
                    ownership = "remote",
                    phase = phase,
                    apiBaseUrl = RemoteRuntimeConfigFallbackValidator.normalizeApiBaseUrl(config.remoteBaseUrl),
                    error = error,
                ),
            nativeAvailable = nativeAvailable,
        )

    private fun invalidSnapshot(config: EditableRuntimeConfig, error: ShellRuntimeError) =
        AndroidRuntimeSnapshot(
            configStatus = "invalid",
            config = config,
            initialized = false,
            service =
                RuntimeServiceSnapshot(
                    ownership = if (RuntimeModes.isRemote(config.mode)) "remote" else "local",
                    phase = "stopped",
                    error = error,
                ),
            nativeAvailable = localAvailable(),
        )

    private fun unconfiguredSnapshot(config: EditableRuntimeConfig, error: ShellRuntimeError? = null) =
        AndroidRuntimeSnapshot(
            configStatus = "unconfigured",
            config = config,
            initialized = false,
            service = RuntimeServiceSnapshot("local", if (error == null) "stopped" else "failed", error = error),
            nativeAvailable = localAvailable(),
        )

    private fun AndroidRuntimeSnapshot.withError(error: ShellRuntimeError) =
        copy(
            service = service.copy(error = error),
            nativeAvailable = localAvailable(),
        )

    private fun publish(next: AndroidRuntimeSnapshot): AndroidRuntimeSnapshot {
        val previous = snapshot
        snapshot = next
        if (next.service.ownership == "local" && next.service.phase == "running") {
            autoRestartAttempts = 0
        }
        maybeScheduleAutoRestart(previous, next)
        listeners.forEach { listener ->
            try {
                listener(next)
            } catch (_: RuntimeException) {
                // 单个页面监听失败不能中断 Application 级 core 生命周期。
            }
        }
        return next
    }

    private fun safeLoadConfig(): RuntimeConfigRepository.Loaded =
        try {
            configRepository.load()
        } catch (_: RuntimeException) {
            RuntimeConfigRepository.Loaded.Invalid
        }

    private fun safeSaveConfig(config: EditableRuntimeConfig): Boolean =
        try {
            configRepository.save(config)
        } catch (_: RuntimeException) {
            false
        }

    private fun localAvailable(): Boolean = nativeReady && storagePaths != null

    private fun configUnavailable() =
        ShellRuntimeError(
            ShellErrorCodes.CONFIG_UNAVAILABLE,
            "无法保存运行配置，请检查设备存储状态",
        )

    private fun configInvalid() =
        ShellRuntimeError(ShellErrorCodes.CONFIG_INVALID, "运行配置无效")

    /** 判断错误是否明确说明持久配置字段本身无效，而非运行环境或 native 组件故障。 */
    private fun isPersistedConfigInvalid(error: ShellRuntimeError): Boolean =
        error.code == ShellErrorCodes.CONFIG_INVALID ||
            error.code == ShellErrorCodes.INVALID_BIND_HOST ||
            error.code == ShellErrorCodes.UNSUPPORTED_RUNTIME_MODE

    private fun nativeUnavailable() =
        ShellRuntimeError(
            ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE,
            "Android 本地服务组件不可用，可切换为远端连接模式",
        )

    private fun serviceCrashed() =
        ShellRuntimeError(
            ShellErrorCodes.SERVICE_CRASHED,
            "本地服务意外退出",
        )

    private fun unsupportedMode() =
        ShellRuntimeError(
            ShellErrorCodes.UNSUPPORTED_RUNTIME_MODE,
            "当前运行模式不管理本地 WineStock 服务",
            RuntimeConfigFields.MODE,
        )

    private fun <T> submit(operation: () -> T): CompletableFuture<T> {
        val future = CompletableFuture<T>()
        executor.execute {
            try {
                future.complete(operation())
            } catch (error: Throwable) {
                future.completeExceptionally(error)
            }
        }
        return future
    }

    private data class ValidationAttempt(
        val validation: RuntimeConfigValidationResult,
        val error: ShellRuntimeError?,
    )

    companion object {
        fun create(context: Context): LocalCoreRuntimeManager {
            val appContext = context.applicationContext
            return LocalCoreRuntimeManager(
                nativeClient = JniNativeCoreClient(),
                configRepository = RuntimeConfigStore(appContext),
                storageProvider = { AndroidStoragePaths.prepare(appContext) },
            )
        }
    }
}
