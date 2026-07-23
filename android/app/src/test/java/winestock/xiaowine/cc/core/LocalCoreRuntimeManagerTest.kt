package winestock.xiaowine.cc.core

import java.util.ArrayDeque
import java.util.concurrent.TimeUnit
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import winestock.xiaowine.cc.shell.DEFAULT_RUNTIME_CONFIG
import winestock.xiaowine.cc.shell.EditableRuntimeConfig
import winestock.xiaowine.cc.shell.RuntimeConfigRepository
import winestock.xiaowine.cc.shell.RuntimeModes
import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

class LocalCoreRuntimeManagerTest {
    private val managers = mutableListOf<LocalCoreRuntimeManager>()

    @After
    fun tearDown() {
        managers.forEach(LocalCoreRuntimeManager::shutdownForTests)
    }

    @Test
    fun missingConfigStartsDefaultWithoutPersistingUntilUserApply() {
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Missing)
        val native = FakeNativeCoreClient()
        val manager = manager(native, store)

        val snapshot = manager.getRuntimeSnapshot().await()

        assertEquals("configured", snapshot.configStatus)
        assertEquals("running", snapshot.service.phase)
        assertTrue(snapshot.createdDefault)
        // 自动默认只起服、不写盘；用户保存后才有 Present 配置。
        assertEquals(null, store.current)
        assertEquals(49152, snapshot.config.port)
        assertEquals("127.0.0.1:49152", snapshot.service.boundAddress)
        assertEquals("http://127.0.0.1:49152", snapshot.service.apiBaseUrl)
        assertFalse(snapshot.service.apiBaseUrl.orEmpty().endsWith(":0"))
        assertEquals(1, native.startCalls)
        assertEquals(listOf(0), native.startConfigs.map(EditableRuntimeConfig::port))

        val applied =
            manager
                .applyRuntimeConfig(snapshot.config)
                .await()
        assertTrue(applied.applied)
        assertFalse(applied.snapshot.createdDefault)
        assertEquals(snapshot.config, store.current)
    }

    @Test
    fun persistedSelfHostedPortConflictAllocatesAndPersistsReplacement() {
        val persisted = DEFAULT_RUNTIME_CONFIG.copy(port = 17890)
        val replacement = persisted.copy(port = 49154)
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Present(persisted))
        val native =
            FakeNativeCoreClient().apply {
                startResults +=
                    NativeCallResult.Failure(
                        ShellRuntimeError(ShellErrorCodes.PORT_IN_USE, "端口被占用", "port"),
                    )
                startResults += successState(replacement)
            }
        val manager = manager(native, store)

        val snapshot = manager.getRuntimeSnapshot().await()

        assertEquals(replacement, snapshot.config)
        assertEquals(replacement, store.current)
        assertEquals("127.0.0.1:49154", snapshot.service.boundAddress)
        assertEquals(listOf(17890, 0), native.startConfigs.map(EditableRuntimeConfig::port))
    }

    @Test
    fun selfHostedPortConflictRetriesWithAutomaticallyAllocatedPort() {
        val previous = DEFAULT_RUNTIME_CONFIG.copy(port = 17890)
        val candidate = previous.copy(port = 17891)
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Present(previous))
        val native =
            FakeNativeCoreClient().apply {
                startResults += successState(previous)
                startResults +=
                    NativeCallResult.Failure(
                        ShellRuntimeError(ShellErrorCodes.PORT_IN_USE, "端口被占用", "port"),
                    )
                startResults += successState(candidate.copy(port = 49153))
            }
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()

        val result = manager.applyRuntimeConfig(candidate).await()

        assertTrue(result.applied)
        assertEquals(candidate.copy(port = 49153), result.snapshot.config)
        assertEquals("running", result.snapshot.service.phase)
        assertEquals(null, result.snapshot.service.error)
        assertEquals(candidate.copy(port = 49153), store.current)
        assertEquals(3, native.startCalls)
    }

    @Test
    fun saveFailureAfterCandidateStartStopsCandidateAndRestoresPreviousService() {
        val previous = DEFAULT_RUNTIME_CONFIG.copy(port = 17890)
        val candidate = previous.copy(port = 17892)
        val store =
            FakeConfigRepository(RuntimeConfigRepository.Loaded.Present(previous)).apply {
                rejectSave = { it == candidate }
            }
        val native =
            FakeNativeCoreClient().apply {
                startResults += successState(previous)
                startResults += successState(candidate)
                startResults += successState(previous)
            }
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()

        val result = manager.applyRuntimeConfig(candidate).await()

        assertFalse(result.applied)
        assertEquals(previous, result.snapshot.config)
        assertEquals("running", result.snapshot.service.phase)
        assertEquals(ShellErrorCodes.CONFIG_UNAVAILABLE, result.error?.code)
        assertEquals(previous, store.current)
        assertEquals(2, native.stopCalls)
    }

    @Test
    fun saveFailureAfterConflictReplacementStopsReplacementAndRestoresPreviousService() {
        val previous = DEFAULT_RUNTIME_CONFIG.copy(port = 17890)
        val candidate = previous.copy(port = 17891)
        val replacement = candidate.copy(port = 49153)
        val store =
            FakeConfigRepository(RuntimeConfigRepository.Loaded.Present(previous)).apply {
                rejectSave = { it == replacement }
            }
        val native =
            FakeNativeCoreClient().apply {
                startResults += successState(previous)
                startResults +=
                    NativeCallResult.Failure(
                        ShellRuntimeError(ShellErrorCodes.PORT_IN_USE, "端口被占用", "port"),
                    )
                startResults += successState(replacement)
                startResults += successState(previous)
            }
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()

        val result = manager.applyRuntimeConfig(candidate).await()

        assertFalse(result.applied)
        assertEquals(previous, result.snapshot.config)
        assertEquals("running", result.snapshot.service.phase)
        assertEquals(ShellErrorCodes.CONFIG_UNAVAILABLE, result.error?.code)
        assertEquals(previous, store.current)
        assertEquals(listOf(17890, 17891, 0, 17890), native.startConfigs.map(EditableRuntimeConfig::port))
        assertEquals(2, native.stopCalls)
    }

    @Test
    fun serverModePortConflictDoesNotRetryWithAutomaticPort() {
        val previous = DEFAULT_RUNTIME_CONFIG.copy(port = 17890)
        val candidate =
            previous.copy(
                mode = RuntimeModes.SERVER_MODE,
                bindHost = "0.0.0.0",
                port = 17891,
            )
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Present(previous))
        val native =
            FakeNativeCoreClient().apply {
                startResults += successState(previous)
                startResults +=
                    NativeCallResult.Failure(
                        ShellRuntimeError(ShellErrorCodes.PORT_IN_USE, "端口被占用", "port"),
                    )
                startResults += successState(previous)
            }
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()

        val result = manager.applyRuntimeConfig(candidate).await()

        assertFalse(result.applied)
        assertEquals(ShellErrorCodes.PORT_IN_USE, result.error?.code)
        assertEquals(previous, result.snapshot.config)
        assertEquals(listOf(17890, 17891, 17890), native.startConfigs.map(EditableRuntimeConfig::port))
        assertFalse(
            native.startConfigs.any {
                it.mode == RuntimeModes.SERVER_MODE && it.port == 0
            },
        )
    }

    @Test
    fun nativeUnavailableStillAllowsRemoteConfiguration() {
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Missing)
        val native =
            FakeNativeCoreClient(
                initializeResult =
                    NativeCallResult.Failure(
                        ShellRuntimeError(
                            ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE,
                            "native unavailable",
                        ),
                    ),
            )
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()
        val remote =
            DEFAULT_RUNTIME_CONFIG.copy(
                mode = RuntimeModes.CONNECT_TO_REMOTE,
                remoteBaseUrl = "https://example.com/api/",
            )

        val result = manager.applyRuntimeConfig(remote).await()

        assertTrue(result.applied)
        assertEquals("remote", result.snapshot.service.ownership)
        assertEquals("https://example.com/api", result.snapshot.service.apiBaseUrl)
        assertEquals("https://example.com/api", store.current?.remoteBaseUrl)
        assertFalse(result.snapshot.nativeAvailable)
    }

    private fun manager(
        native: FakeNativeCoreClient,
        store: FakeConfigRepository,
    ): LocalCoreRuntimeManager =
        LocalCoreRuntimeManager(
            nativeClient = native,
            configRepository = store,
            storageProvider = {
                NativeCallResult.Success(
                    NativeStoragePaths("C:/data/winestock.sqlite", "C:/data/files"),
                )
            },
        ).also(managers::add)

    private fun <T> java.util.concurrent.CompletableFuture<T>.await(): T =
        get(5, TimeUnit.SECONDS)

    private class FakeConfigRepository(initial: RuntimeConfigRepository.Loaded) :
        RuntimeConfigRepository {
        var current: EditableRuntimeConfig? =
            (initial as? RuntimeConfigRepository.Loaded.Present)?.config
        var loaded: RuntimeConfigRepository.Loaded = initial
        var rejectSave: (EditableRuntimeConfig) -> Boolean = { false }

        override fun load(): RuntimeConfigRepository.Loaded = loaded

        override fun save(config: EditableRuntimeConfig): Boolean {
            if (rejectSave(config)) return false
            current = config
            loaded = RuntimeConfigRepository.Loaded.Present(config)
            return true
        }
    }

    private class FakeNativeCoreClient(
        private val initializeResult: NativeCallResult<Unit> = NativeCallResult.Success(Unit),
    ) : NativeCoreClient {
        val startResults = ArrayDeque<NativeCallResult<NativeServiceState>>()
        val startConfigs = mutableListOf<EditableRuntimeConfig>()
        var startCalls = 0
        var stopCalls = 0
        private var state = NativeServiceState("stopped", null, null, null)

        override fun initialize(): NativeCallResult<Unit> = initializeResult

        override fun defaultRuntimeConfig(): NativeCallResult<EditableRuntimeConfig> =
            NativeCallResult.Success(DEFAULT_RUNTIME_CONFIG)

        override fun validateRuntimeConfig(
            config: EditableRuntimeConfig,
            storage: NativeStoragePaths,
        ): NativeCallResult<NativeValidationResult> =
            NativeCallResult.Success(
                NativeValidationResult(true, emptyMap(), config),
            )

        override fun startLocalService(
            config: EditableRuntimeConfig,
            storage: NativeStoragePaths,
        ): NativeCallResult<NativeServiceState> {
            startCalls += 1
            startConfigs += config
            val result =
                if (startResults.isEmpty()) successState(config) else startResults.removeFirst()
            if (result is NativeCallResult.Success) state = result.value
            return result
        }

        override fun stopLocalService(): NativeCallResult<NativeServiceState> {
            stopCalls += 1
            state = NativeServiceState("stopped", null, null, null)
            return NativeCallResult.Success(state)
        }

        override fun restartLocalService(
            config: EditableRuntimeConfig,
            storage: NativeStoragePaths,
        ): NativeCallResult<NativeServiceState> {
            stopLocalService()
            return startLocalService(config, storage)
        }

        override fun getRuntimeState(): NativeCallResult<NativeServiceState> =
            NativeCallResult.Success(state)

        fun successState(config: EditableRuntimeConfig): NativeCallResult.Success<NativeServiceState> =
            NativeCallResult.Success(
                NativeServiceState(
                    phase = "running",
                    boundAddress = "127.0.0.1:${actualPort(config)}",
                    apiBaseUrl = "http://127.0.0.1:${actualPort(config)}",
                    error = null,
                ),
            )

        private fun actualPort(config: EditableRuntimeConfig): Int =
            if (config.port == 0) 49152 else config.port
    }
}
