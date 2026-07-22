package winestock.xiaowine.cc.core

import org.json.JSONObject
import winestock.xiaowine.cc.shell.EditableRuntimeConfig
import winestock.xiaowine.cc.shell.ShellErrorCodes
import winestock.xiaowine.cc.shell.ShellRuntimeError

const val NATIVE_PROTOCOL_VERSION = 1

data class NativeStoragePaths(
    val databasePath: String,
    val filesDir: String,
) {
    fun toJson(): JSONObject =
        JSONObject()
            .put("databasePath", databasePath)
            .put("filesDir", filesDir)
}

data class NativeValidationResult(
    val valid: Boolean,
    val fieldErrors: Map<String, List<String>>,
    val normalizedConfig: EditableRuntimeConfig?,
)

data class NativeServiceState(
    val phase: String,
    val boundAddress: String?,
    val apiBaseUrl: String?,
    val error: ShellRuntimeError?,
)

sealed interface NativeCallResult<out T> {
    data class Success<T>(val value: T) : NativeCallResult<T>
    data class Failure(val error: ShellRuntimeError) : NativeCallResult<Nothing>
}

/** native protocol v1 JSON 的唯一编解码入口。 */
object NativeContract {
    fun requestJson(config: EditableRuntimeConfig, storage: NativeStoragePaths): String =
        JSONObject()
            .put("nativeProtocolVersion", NATIVE_PROTOCOL_VERSION)
            .put("config", config.toJson())
            .put("storage", storage.toJson())
            .toString()

    fun parseInitialize(raw: String?): NativeCallResult<Unit> =
        parse(raw) { result ->
            if (!result.optBoolean("initialized", false)) {
                throw IllegalArgumentException("native engine 未初始化")
            }
            Unit
        }

    fun parseDefaultConfig(raw: String?): NativeCallResult<EditableRuntimeConfig> =
        parse(raw) { result -> requireConfig(result) }

    fun parseValidation(raw: String?): NativeCallResult<NativeValidationResult> =
        parse(raw) { result ->
            val fieldErrorsJson = result.optJSONObject("fieldErrors") ?: JSONObject()
            val fieldErrors = linkedMapOf<String, List<String>>()
            for (field in fieldErrorsJson.keys()) {
                val messages = fieldErrorsJson.optJSONArray(field) ?: continue
                fieldErrors[field] =
                    buildList {
                        for (index in 0 until messages.length()) {
                            val message = messages.optString(index)
                            if (message.isNotBlank()) add(message)
                        }
                    }
            }
            NativeValidationResult(
                valid = result.optBoolean("valid", false),
                fieldErrors = fieldErrors,
                normalizedConfig = result.optJSONObject("normalizedConfig")?.let(::requireConfig),
            )
        }

    fun parseServiceState(raw: String?): NativeCallResult<NativeServiceState> =
        parse(raw) { result ->
            NativeServiceState(
                phase = result.getString("phase"),
                boundAddress = result.optionalString("boundAddress"),
                apiBaseUrl = result.optionalString("apiBaseUrl"),
                error = result.optJSONObject("error")?.let(::parseError),
            )
        }

    private fun <T> parse(raw: String?, mapper: (JSONObject) -> T): NativeCallResult<T> {
        if (raw.isNullOrBlank()) return NativeCallResult.Failure(nativeUnavailable())
        return try {
            val envelope = JSONObject(raw)
            if (envelope.optInt("nativeProtocolVersion", -1) != NATIVE_PROTOCOL_VERSION) {
                return NativeCallResult.Failure(
                    ShellRuntimeError(
                        ShellErrorCodes.BRIDGE_VERSION_MISMATCH,
                        "Android native 协议版本不兼容",
                    ),
                )
            }
            if (!envelope.optBoolean("ok", false)) {
                return NativeCallResult.Failure(
                    envelope.optJSONObject("error")?.let(::parseError)
                        ?: ShellRuntimeError(
                            ShellErrorCodes.SERVICE_START_FAILED,
                            "Android native 调用失败",
                        ),
                )
            }
            val result = envelope.optJSONObject("result")
                ?: return NativeCallResult.Failure(invalidPayload())
            NativeCallResult.Success(mapper(result))
        } catch (_: Exception) {
            NativeCallResult.Failure(invalidPayload())
        }
    }

    private fun requireConfig(json: JSONObject): EditableRuntimeConfig =
        EditableRuntimeConfig.fromJsonOrNull(json)
            ?: throw IllegalArgumentException("native config 无效")

    private fun parseError(json: JSONObject): ShellRuntimeError =
        ShellRuntimeError(
            code = json.optString("code", ShellErrorCodes.SERVICE_START_FAILED),
            message = json.optString("message", "Android 本地服务调用失败"),
            field = json.optionalString("field"),
        )

    private fun JSONObject.optionalString(name: String): String? =
        if (has(name) && !isNull(name)) optString(name).takeIf(String::isNotBlank) else null

    private fun nativeUnavailable() =
        ShellRuntimeError(
            ShellErrorCodes.NATIVE_LIBRARY_UNAVAILABLE,
            "Android 本地服务组件无响应，可切换为远端连接模式",
        )

    private fun invalidPayload() =
        ShellRuntimeError(
            ShellErrorCodes.INVALID_BRIDGE_PAYLOAD,
            "Android native 返回结构无效",
        )
}
