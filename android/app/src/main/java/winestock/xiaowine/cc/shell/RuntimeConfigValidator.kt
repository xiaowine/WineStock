package winestock.xiaowine.cc.shell

import android.net.Uri

/**
 * 运行配置校验与 API 地址规范化。
 *
 * 本文件是 shared 校验规则在 Android 侧的镜像，语义对齐 frontend/src/shell/web.ts 与
 * frontend/src/api/runtime-config.ts。端上原生 Rust 服务落地后应改为委托 winestock_shared。
 */
object RuntimeConfigValidator {

    /** 单字段校验结果集合，键为稳定字段名，值为错误文案列表。 */
    data class Result(val fieldErrors: Map<String, List<String>>) {
        val valid: Boolean get() = fieldErrors.isEmpty()
    }

    /** 按 web.ts 的规则执行即时表单校验。 */
    fun validate(config: EditableRuntimeConfig): Result {
        val errors = linkedMapOf<String, MutableList<String>>()
        fun add(field: String, message: String) {
            errors.getOrPut(field) { mutableListOf() }.add(message)
        }

        if (config.mode !in RuntimeModes.ALL) {
            add(RuntimeConfigFields.MODE, "请选择有效的运行方式")
        }
        if (config.port < 1 || config.port > 65535) {
            add(RuntimeConfigFields.PORT, "端口必须是 1 到 65535 之间的整数")
        }

        if (RuntimeModes.isRemote(config.mode)) {
            if (config.remoteBaseUrl.isBlank()) {
                add(RuntimeConfigFields.REMOTE_BASE_URL, "请输入远程服务 API 地址")
            } else {
                val normalizeError = normalizeApiBaseUrlError(config.remoteBaseUrl)
                if (normalizeError != null) {
                    add(RuntimeConfigFields.REMOTE_BASE_URL, normalizeError)
                }
            }
        } else if (config.bindHost.isBlank()) {
            add(RuntimeConfigFields.BIND_HOST, "请输入本地服务监听地址")
        } else if (!isIpAddress(config.bindHost.trim())) {
            add(RuntimeConfigFields.BIND_HOST, "监听地址必须是有效的 IPv4 或 IPv6 地址")
        }

        return Result(errors.mapValues { it.value.toList() })
    }

    /**
     * 规范化 API 根地址，规则与前端 normalizeApiBaseUrl 一致：
     * 只允许 http/https，禁止全接口监听地址、凭据、查询串和 hash，去掉末尾斜杠。
     * 校验失败时返回 null。
     */
    fun normalizeApiBaseUrl(value: String): String? {
        val uri =
            try {
                Uri.parse(value.trim())
            } catch (_: Exception) {
                return null
            }
        val scheme = uri.scheme?.lowercase()
        if (scheme != "http" && scheme != "https") {
            return null
        }
        val host = uri.host ?: return null
        if (host == "0.0.0.0" || host == "::" || host == "[::]") {
            return null
        }
        // 禁止携带凭据、查询参数或 fragment。
        if (uri.userInfo != null || uri.query != null || uri.fragment != null) {
            return null
        }

        val portPart = if (uri.port != -1) ":${uri.port}" else ""
        val pathPart = (uri.path ?: "").trimEnd('/')
        return "$scheme://$host$portPart$pathPart"
    }

    private fun normalizeApiBaseUrlError(value: String): String? {
        val trimmed = value.trim()
        val uri =
            try {
                Uri.parse(trimmed)
            } catch (_: Exception) {
                return "远程服务地址无效"
            }
        val scheme = uri.scheme?.lowercase()
        if (scheme != "http" && scheme != "https") {
            return "WineStock 服务地址必须使用 http 或 https"
        }
        val host = uri.host
        if (host == null) {
            return "远程服务地址无效"
        }
        if (host == "0.0.0.0" || host == "::" || host == "[::]") {
            return "全接口监听地址不能作为前端访问地址"
        }
        if (uri.userInfo != null || uri.query != null || uri.fragment != null) {
            return "WineStock 服务地址不能包含凭据、查询参数或 hash"
        }
        return null
    }

    /** 判断字符串是否为合法 IPv4 或 IPv6 地址，规则对齐 web.ts 的 isIpAddress。 */
    fun isIpAddress(value: String): Boolean {
        if (value.contains(":")) {
            return Regex("^[0-9a-fA-F:]+$").matches(value)
        }
        val segments = value.split(".")
        return segments.size == 4 &&
            segments.all { segment ->
                Regex("^\\d{1,3}$").matches(segment) && segment.toInt() <= 255
            }
    }
}
