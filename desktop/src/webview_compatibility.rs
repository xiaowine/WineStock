//! Desktop 启动前的 WebView2 运行时版本门禁。
//!
//! Tauri 没有跨平台的 WebView 引擎版本查询 API；Windows 侧使用 Microsoft
//! WebView2 Loader 官方 API 读取 Evergreen Runtime 版本。该检查发生在主窗口显示前，
//! 不依赖前端或 WebView 自身执行 JavaScript，也不读取注册表。

use std::cmp::Ordering;

// 与 Android Shell 的 Chromium 主版本门槛 M111 对齐；补丁号不设人为下限。
pub const MINIMUM_WEBVIEW2_VERSION: [u32; 4] = [111, 0, 0, 0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebViewRuntimeInfo {
    pub version: Option<String>,
    pub supported: bool,
}

pub fn check() -> WebViewRuntimeInfo {
    #[cfg(target_os = "windows")]
    {
        let version = read_installed_version();
        let supported = version
            .as_deref()
            .and_then(parse_version)
            .map(|value| value.cmp(&MINIMUM_WEBVIEW2_VERSION) != Ordering::Less)
            .unwrap_or(false);
        return WebViewRuntimeInfo { version, supported };
    }

    // macOS/Linux 的 WebKit 由系统提供，当前没有引入额外的 GTK/Objective-C 绑定；
    // 不把 Windows WebView2 API 误用于其它平台，也不因此阻断尚未实现门禁的平台。
    #[cfg(not(target_os = "windows"))]
    WebViewRuntimeInfo {
        version: None,
        supported: true,
    }
}

#[cfg(target_os = "windows")]
fn read_installed_version() -> Option<String> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Com::CoTaskMemFree;

    let mut version_ptr = PWSTR::null();
    let result = unsafe {
        webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString(
            PCWSTR::null(),
            &mut version_ptr,
        )
    };
    if result.is_err() || version_ptr.is_null() {
        return None;
    }

    let value = match unsafe { version_ptr.to_string() } {
        Ok(value) => value,
        Err(_) => {
            unsafe { CoTaskMemFree(Some(version_ptr.0.cast())) };
            return None;
        }
    };
    unsafe { CoTaskMemFree(Some(version_ptr.0.cast())) };
    Some(value)
}

fn parse_version(value: &str) -> Option<[u32; 4]> {
    let parts = value.trim().split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    let mut parsed = [0; 4];
    for (index, part) in parts.into_iter().enumerate() {
        parsed[index] = part.parse().ok()?;
    }
    Some(parsed)
}

#[cfg(test)]
mod tests {
    use super::{parse_version, MINIMUM_WEBVIEW2_VERSION};

    #[test]
    fn accepts_m111_minimum_version() {
        assert!(parse_version("111.0.1661.54").unwrap() >= MINIMUM_WEBVIEW2_VERSION);
    }

    #[test]
    fn rejects_older_or_malformed_versions() {
        assert!(parse_version("110.0.1587.57").unwrap() < MINIMUM_WEBVIEW2_VERSION);
        assert_eq!(parse_version("111.0.1661"), None);
    }
}
