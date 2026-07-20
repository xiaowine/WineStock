package winestock.xiaowine.cc.shell

import org.json.JSONArray
import org.json.JSONObject

/**
 * 构造 Shell Bridge v1 运行快照。
 *
 * 输出的 JSON 必须通过 frontend/src/shell/contract.ts 的 assertCompatibleRuntimeSnapshot 校验：
 * protocolVersion 固定为 1，platform 为 "android"，capabilities 六个布尔字段齐全。
 * 本文件不管理服务生命周期，只根据配置状态派生前端可消费的快照结构。
 */
object RuntimeSnapshotFactory {

    const val PROTOCOL_VERSION = 1
    const val PLATFORM = "android"

    /**
     * Android 传输层当前能力。
     *
     * 端上原生 Axum 尚未实现，因此本地服务启停能力为 false，serverMode 也禁用；
     * 待 core 提供可停止的本地服务句柄后再放开这些能力。
     * nativeBack 暂为 false：本次未实现“前端优先处理返回键”的协商协议，
     * Activity 侧仍自行处理 WebView 返回，不声明未兑现的能力。
     */
    private fun capabilities(): JSONObject =
        JSONObject()
            .put("startLocalService", false)
            .put("stopLocalService", false)
            .put("restartLocalService", false)
            .put("nativeBack", false)
            .put("openExternal", true)
            .put("serverMode", false)

    /** 已配置且校验通过的运行快照。 */
    fun configured(config: EditableRuntimeConfig): JSONObject {
        val remote = RuntimeModes.isRemote(config.mode)
        val apiBaseUrl =
            if (remote) {
                RuntimeConfigValidator.normalizeApiBaseUrl(config.remoteBaseUrl)
            } else {
                "http://127.0.0.1:${config.port}"
            }
        val service =
            JSONObject()
                .put("ownership", if (remote) "remote" else "local")
                .put("phase", "running")
        if (apiBaseUrl != null) {
            service.put("apiBaseUrl", apiBaseUrl)
        }
        if (!remote) {
            service.put("boundAddress", "${config.bindHost}:${config.port}")
        }
        return baseSnapshot("configured", config, createdDefault = false, service = service)
    }

    /** 配置无效但保留用户草稿，让前端进入设置页修复。 */
    fun invalid(config: EditableRuntimeConfig, message: String): JSONObject {
        val service =
            JSONObject()
                .put("ownership", if (RuntimeModes.isRemote(config.mode)) "remote" else "local")
                .put("phase", "stopped")
                .put(
                    "error",
                    JSONObject()
                        .put("code", ShellErrorCodes.CONFIG_INVALID)
                        .put("message", message),
                )
        return baseSnapshot("invalid", config, createdDefault = false, service = service)
    }

    /** 从未配置，等待首次设置。 */
    fun unconfigured(): JSONObject {
        val service =
            JSONObject()
                .put("ownership", "local")
                .put("phase", "stopped")
        return baseSnapshot(
            configStatus = "unconfigured",
            config = DEFAULT_RUNTIME_CONFIG,
            createdDefault = false,
            service = service,
        )
    }

    /**
     * 在现有快照基础上附加一次性运行错误，用于本地模式不支持等场景。
     * 不改变配置状态，只补充 service.error。
     */
    fun withError(snapshot: JSONObject, code: String, message: String): JSONObject {
        val next = JSONObject(snapshot.toString())
        val service = next.getJSONObject("service")
        service.put(
            "error",
            JSONObject().put("code", code).put("message", message),
        )
        return next
    }

    private fun baseSnapshot(
        configStatus: String,
        config: EditableRuntimeConfig,
        createdDefault: Boolean,
        service: JSONObject,
    ): JSONObject =
        JSONObject()
            .put("protocolVersion", PROTOCOL_VERSION)
            .put("platform", PLATFORM)
            .put("configStatus", configStatus)
            .put("config", config.toJson())
            .put("createdDefault", createdDefault)
            .put("service", service)
            .put("capabilities", capabilities())
}
