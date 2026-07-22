package winestock.xiaowine.cc.shell

import org.json.JSONObject

data class RuntimeServiceSnapshot(
    val ownership: String,
    val phase: String,
    val apiBaseUrl: String? = null,
    val boundAddress: String? = null,
    val error: ShellRuntimeError? = null,
)

data class AndroidRuntimeSnapshot(
    val configStatus: String,
    val config: EditableRuntimeConfig,
    val createdDefault: Boolean,
    val service: RuntimeServiceSnapshot,
    /** native 加载成功才开放本地生命周期 capability；server-mode 仍保持禁用。 */
    val nativeAvailable: Boolean,
)

/** 把 Application 级 manager 状态投影为 Shell Bridge v1 JSON。 */
object RuntimeSnapshotFactory {
    const val PROTOCOL_VERSION = 1
    const val PLATFORM = "android"

    fun toJson(snapshot: AndroidRuntimeSnapshot, nativeBackSupported: Boolean): JSONObject {
        val localLifecycleAvailable =
            snapshot.nativeAvailable && snapshot.service.ownership == "local"
        val service =
            JSONObject()
                .put("ownership", snapshot.service.ownership)
                .put("phase", snapshot.service.phase)
        snapshot.service.apiBaseUrl?.let { service.put("apiBaseUrl", it) }
        snapshot.service.boundAddress?.let { service.put("boundAddress", it) }
        snapshot.service.error?.let { service.put("error", errorJson(it)) }

        return JSONObject()
            .put("protocolVersion", PROTOCOL_VERSION)
            .put("platform", PLATFORM)
            .put("configStatus", snapshot.configStatus)
            .put("config", snapshot.config.toJson())
            .put("createdDefault", snapshot.createdDefault)
            .put("service", service)
            .put(
                "capabilities",
                JSONObject()
                    .put("startLocalService", localLifecycleAvailable)
                    .put("stopLocalService", localLifecycleAvailable)
                    .put("restartLocalService", localLifecycleAvailable)
                    .put("nativeBack", nativeBackSupported)
                    .put("openExternal", true)
                    .put("serverMode", false),
            )
    }

    fun errorJson(error: ShellRuntimeError): JSONObject =
        JSONObject()
            .put("code", error.code)
            .put("message", error.message)
            .also { json -> error.field?.let { json.put("field", it) } }
}
