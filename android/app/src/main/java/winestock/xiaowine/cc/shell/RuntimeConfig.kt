package winestock.xiaowine.cc.shell

import org.json.JSONObject

/**
 * Android shell 侧的运行配置模型与常量。
 *
 * 本文件属于 Shell Bridge 传输层，镜像 frontend/src/shell/contract.ts 的字段语义，
 * 也对应 winestock_shared::AppConfig 的运行参数。它不启动 Axum、不读取业务数据。
 * 权威校验由 android/native -> winestock_shared 执行；Kotlin 只保留传输 DTO 与 native
 * 不可用时连接远端所需的最小降级规则。
 */

/** frontend 可编辑并交给 Shell 应用的运行配置四字段。 */
data class EditableRuntimeConfig(
    val mode: String,
    val bindHost: String,
    val port: Int,
    val remoteBaseUrl: String,
) {
    /** 序列化为 Shell 快照和事件信封使用的 JSON 结构。 */
    fun toJson(): JSONObject =
        JSONObject()
            .put("mode", mode)
            .put("bindHost", bindHost)
            .put("port", port)
            .put("remoteBaseUrl", remoteBaseUrl)

    companion object {
        /**
         * 从任意 JSON 值解析可编辑配置；仅接受结构与类型都合法的对象，
         * 与 web.ts 的 isEditableRuntimeConfig 判定保持一致。
         */
        fun fromJsonOrNull(value: Any?): EditableRuntimeConfig? {
            val json = value as? JSONObject ?: return null
            // 三个文本字段必须存在且为 String，与 web.ts 的 isEditableRuntimeConfig 判定一致。
            val mode = requireString(json, "mode") ?: return null
            val bindHost = requireString(json, "bindHost") ?: return null
            val remoteBaseUrl = requireString(json, "remoteBaseUrl") ?: return null
            // port 必须是 JSON number；opt 系列在缺失时返回默认值，因此显式检查存在性与类型。
            if (!json.has("port") || json.get("port") !is Number) {
                return null
            }
            return EditableRuntimeConfig(
                mode = mode,
                bindHost = bindHost,
                port = json.getInt("port"),
                remoteBaseUrl = remoteBaseUrl,
            )
        }

        /** 字段存在且为 JSON String 时返回其值，否则返回 null。 */
        private fun requireString(json: JSONObject, name: String): String? {
            val raw = if (json.has(name)) json.get(name) else return null
            return raw as? String
        }
    }
}

/** native 尚不可加载时用于保持设置页可打开的应急默认值；正常路径使用 shared 返回的默认配置。 */
val DEFAULT_RUNTIME_CONFIG =
    EditableRuntimeConfig(
        mode = RuntimeModes.SELF_HOSTED,
        bindHost = "127.0.0.1",
        port = 17890,
        remoteBaseUrl = "",
    )

/** RuntimeMode 稳定字符串，与 shared 的 kebab-case 序列化一致。 */
object RuntimeModes {
    const val SELF_HOSTED = "self-hosted"
    const val CLIENT_ONLY = "client-only"
    const val CONNECT_TO_REMOTE = "connect-to-remote"
    const val SERVER_MODE = "server-mode"

    val ALL = setOf(SELF_HOSTED, CLIENT_ONLY, CONNECT_TO_REMOTE, SERVER_MODE)

    /** 仅连接远端服务的客户端模式。 */
    fun isRemote(mode: String): Boolean = mode == CLIENT_ONLY || mode == CONNECT_TO_REMOTE

    /** 需要本地 Axum 的模式。 */
    fun isLocal(mode: String): Boolean = mode == SELF_HOSTED || mode == SERVER_MODE
}

/** Shell Bridge v1 稳定错误码，与 docs/shell-bridge.md 保持一致。 */
object ShellErrorCodes {
    const val BRIDGE_VERSION_MISMATCH = "bridge_version_mismatch"
    const val CONFIG_UNAVAILABLE = "config_unavailable"
    const val CONFIG_INVALID = "config_invalid"
    const val STORAGE_UNAVAILABLE = "storage_unavailable"
    const val DATABASE_OPEN_FAILED = "database_open_failed"
    const val MIGRATION_FAILED = "migration_failed"
    const val INVALID_BIND_HOST = "invalid_bind_host"
    const val PORT_IN_USE = "port_in_use"
    const val SERVICE_START_FAILED = "service_start_failed"
    const val SERVICE_CRASHED = "service_crashed"
    const val NATIVE_LIBRARY_UNAVAILABLE = "native_library_unavailable"
    const val UNSUPPORTED_RUNTIME_MODE = "unsupported_runtime_mode"
    const val INVALID_BRIDGE_PAYLOAD = "invalid_bridge_payload"
}

/** 运行配置字段稳定名称。 */
object RuntimeConfigFields {
    const val MODE = "mode"
    const val BIND_HOST = "bindHost"
    const val PORT = "port"
    const val REMOTE_BASE_URL = "remoteBaseUrl"
}

/** Shell Bridge v1 字段校验结果。 */
data class RuntimeConfigValidationResult(
    val fieldErrors: Map<String, List<String>>,
    val normalizedConfig: EditableRuntimeConfig? = null,
) {
    val valid: Boolean get() = fieldErrors.isEmpty()
}

/** Shell Bridge 可安全返回的稳定运行错误。 */
data class ShellRuntimeError(
    val code: String,
    val message: String,
    val field: String? = null,
)
