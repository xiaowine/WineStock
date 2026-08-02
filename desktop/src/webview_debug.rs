//! Desktop Windows WebView2 的开发期 CDP 调试配置。
//!
//! WebView2 只在创建 CoreWebView2 环境前读取浏览器参数，因此必须由 Rust
//! Shell 在创建 Tauri 窗口前设置。Debug 构建默认只监听本机 9222 端口；
//! Release 构建无条件清理外部参数，避免正式包被环境变量注入调试能力。

const WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
pub const CDP_PORT_ENVIRONMENT_VARIABLE: &str = "WINESTOCK_WEBVIEW2_CDP_PORT";
pub const DEFAULT_CDP_PORT: u16 = 9222;

/// 在 WebView2 创建前应用当前构建类型的 CDP 策略。
pub fn configure() {
    #[cfg(target_os = "windows")]
    {
        // 不接受调用进程直接注入的任意 Chromium 参数；Debug 也只由下方
        // 的端口配置生成固定参数，Release 则保持为空。
        std::env::remove_var(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS);

        #[cfg(debug_assertions)]
        configure_debug_port();
    }
}

#[cfg(all(target_os = "windows", debug_assertions))]
fn configure_debug_port() {
    let port = match std::env::var(CDP_PORT_ENVIRONMENT_VARIABLE) {
        Ok(value) => parse_port(&value).unwrap_or_else(|| {
            eprintln!(
                "{CDP_PORT_ENVIRONMENT_VARIABLE} 无效，将使用默认 WebView2 CDP 端口 {DEFAULT_CDP_PORT}"
            );
            DEFAULT_CDP_PORT
        }),
        Err(_) => DEFAULT_CDP_PORT,
    };

    let arguments = format!("--remote-debugging-address=127.0.0.1 --remote-debugging-port={port}");
    std::env::set_var(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, arguments);
    eprintln!("WebView2 CDP 已启用：127.0.0.1:{port}");
}

#[cfg(any(debug_assertions, test))]
fn parse_port(value: &str) -> Option<u16> {
    let port = value.trim().parse::<u16>().ok()?;
    (port > 0).then_some(port)
}

#[cfg(test)]
mod tests {
    use super::{parse_port, DEFAULT_CDP_PORT};

    #[test]
    fn accepts_valid_cdp_ports() {
        assert_eq!(parse_port("9222"), Some(DEFAULT_CDP_PORT));
        assert_eq!(parse_port(" 12345 "), Some(12345));
        assert_eq!(parse_port("65535"), Some(65535));
    }

    #[test]
    fn rejects_invalid_cdp_ports() {
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("65536"), None);
        assert_eq!(parse_port("not-a-port"), None);
    }
}
