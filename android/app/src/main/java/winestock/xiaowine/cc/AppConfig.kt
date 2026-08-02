package winestock.xiaowine.cc

/** 集中管理 Android shell 的可调常量。 */
internal object AppConfig {
    /**
     * 打包前端资源的受信任本地 origin。
     *
     * 使用 ICANN 保留、永不进入公网 DNS 的 .internal 顶级域，仅供本 App 内部使用；
     * 即便请求意外走到网络也无法解析到公网主机。scheme 保持 https 以维持 secure context，
     * 使前端的 Web Locks、IndexedDB 等能力可用。
     * 本值同时作为 WebViewAssetLoader 域名和 Shell Bridge 允许的唯一 origin。
     */
    const val TRUSTED_HOST = "winestock.internal"

    /** 前端入口地址，由 WebViewAssetLoader 拦截并从 assets/frontend 提供。 */
    const val FRONTEND_HOME_URL = "https://$TRUSTED_HOST/"

    /** Shell Bridge 消息通道允许的 origin 规则，仅限受信任前端 origin。 */
    const val TRUSTED_ORIGIN = "https://$TRUSTED_HOST"

    /** Shell Bridge 等待前端完成握手的最长时间；超时由原生壳显示失败提示。 */
    const val SHELL_BRIDGE_READY_TIMEOUT_MS = 8_000L

    /** Android 等待前端结算一次原生返回请求的最长时间；超时后重新读取 WebView history 并 fallback。 */
    const val NATIVE_BACK_RESPONSE_TIMEOUT_MS = 400L
}
