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
    fun missingConfigStartsCoreThenCommitsSharedDefault() {
        val store = FakeConfigRepository(RuntimeConfigRepository.Loaded.Missing)
        val native = FakeNativeCoreClient()
        val manager = manager(native, store)

        val snapshot = manager.getRuntimeSnapshot().await()

        assertEquals("configured", snapshot.configStatus)
        assertEquals("running", snapshot.service.phase)
        assertTrue(snapshot.createdDefault)
        assertEquals(DEFAULT_RUNTIME_CONFIG, store.current)
        assertEquals(1, native.startCalls)
    }

    @Test
    fun candidateStartFailureRestoresPreviousRunningServiceWithoutSavingCandidate() {
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
                startResults += successState(previous)
            }
        val manager = manager(native, store)
        manager.getRuntimeSnapshot().await()

        val result = manager.applyRuntimeConfig(candidate).await()

        assertFalse(result.applied)
        assertEquals(previous, result.snapshot.config)
        assertEquals("running", result.snapshot.service.phase)
        assertEquals(ShellErrorCodes.PORT_IN_USE, result.snapshot.service.error?.code)
        assertEquals(previous, store.current)
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
                    boundAddress = "127.0.0.1:${config.port}",
                    apiBaseUrl = "http://127.0.0.1:${config.port}",
                    error = null,
                ),
            )
    }
}
