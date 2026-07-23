package winestock.xiaowine.cc.shell

import java.net.URI

/**
 * native library 不可用时的最小远端降级校验。
 *
 * 本对象不校验或启动本地模式，也不镜像 shared 的完整规则；正常路径始终委托 Rust/shared。
 */
object RemoteRuntimeConfigFallbackValidator {

    fun validate(config: EditableRuntimeConfig): RuntimeConfigValidationResult {
        val errors = linkedMapOf<String, List<String>>()
        if (!RuntimeModes.isRemote(config.mode)) {
            errors[RuntimeConfigFields.MODE] = listOf("本地服务组件不可用，请改用远端连接模式")
        }
        val automaticSelfHostedPort =
            config.mode == RuntimeModes.SELF_HOSTED && config.port == 0
        if (config.port !in 1..65535 && !automaticSelfHostedPort) {
            errors[RuntimeConfigFields.PORT] = listOf("端口必须是 1 到 65535 之间的整数")
        }
        val normalized = normalizeApiBaseUrl(config.remoteBaseUrl)
        if (RuntimeModes.isRemote(config.mode) && normalized == null) {
            errors[RuntimeConfigFields.REMOTE_BASE_URL] = listOf(remoteUrlError(config.remoteBaseUrl))
        }
        return RuntimeConfigValidationResult(
            fieldErrors = errors,
            normalizedConfig =
                if (errors.isEmpty() && normalized != null) {
                    config.copy(remoteBaseUrl = normalized)
                } else {
                    null
                },
        )
    }

    /** 规范化前端实际使用的 API 根地址；禁止凭据、query、fragment 和全接口地址。 */
    fun normalizeApiBaseUrl(value: String): String? {
        val uri = parseHttpUri(value) ?: return null
        if (uri.rawUserInfo != null || uri.rawQuery != null || uri.rawFragment != null) return null
        val host = uri.host ?: return null
        if (isUnspecifiedHost(host)) return null
        val authorityHost =
            if (host.startsWith('[') && host.endsWith(']')) {
                host
            } else if (host.contains(':')) {
                "[$host]"
            } else {
                host
            }
        val port = if (uri.port >= 0) ":${uri.port}" else ""
        val path = (uri.rawPath ?: "").trimEnd('/')
        return "${uri.scheme.lowercase()}://$authorityHost$port$path"
    }

    /** 外部链接只做 scheme、host、凭据校验；允许路径、query 与 fragment。 */
    fun isSafeExternalHttpUrl(value: String): Boolean {
        val uri = parseHttpUri(value) ?: return false
        val host = uri.host ?: return false
        return uri.rawUserInfo == null && !isUnspecifiedHost(host)
    }

    private fun remoteUrlError(value: String): String {
        if (value.isBlank()) return "请输入远端服务 API 地址"
        val uri = parseHttpUri(value) ?: return "远端服务地址必须使用 http 或 https"
        if (uri.host == null) return "远端服务地址必须包含主机"
        if (isUnspecifiedHost(uri.host)) return "全接口监听地址不能作为前端访问地址"
        if (uri.rawUserInfo != null || uri.rawQuery != null || uri.rawFragment != null) {
            return "远端服务地址不能包含凭据、查询参数或 hash"
        }
        return "远端服务地址无效"
    }

    private fun parseHttpUri(value: String): URI? =
        try {
            URI(value.trim()).takeIf { uri ->
                uri.isAbsolute &&
                    (uri.scheme.equals("http", ignoreCase = true) ||
                        uri.scheme.equals("https", ignoreCase = true))
            }
        } catch (_: Exception) {
            null
        }

    private fun isUnspecifiedHost(host: String): Boolean =
        host == "0.0.0.0" || host == "::" || host == "[::]" || host == "0:0:0:0:0:0:0:0"
}
