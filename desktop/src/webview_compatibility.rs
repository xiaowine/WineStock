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
    pub failure: Option<WebViewRuntimeFailure>,
}

/// WebView2 门卫的内部失败分类；用户展示码由 [`Self::diagnostic_code`] 提供。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebViewRuntimeFailure {
    Missing,
    VersionTooOld,
    VersionInvalid,
    VersionCheckFailed,
    ForcedBlock,
}

impl WebViewRuntimeFailure {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Missing => "webview2_missing",
            Self::VersionTooOld => "webview2_version_too_old",
            Self::VersionInvalid => "webview2_version_invalid",
            Self::VersionCheckFailed => "webview2_version_check_failed",
            Self::ForcedBlock => "webview2_forced_block",
        }
    }

    /// 面向用户和反馈信息的稳定诊断码；与内部日志/协议码分开维护。
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::Missing => "WEBVIEW2_MISSING",
            Self::VersionTooOld => "WEBVIEW2_VERSION_TOO_OLD",
            Self::VersionInvalid => "WEBVIEW2_VERSION_INVALID",
            Self::VersionCheckFailed => "WEBVIEW2_VERSION_CHECK_FAILED",
            Self::ForcedBlock => "WEBVIEW2_FORCED_BLOCK",
        }
    }
}

pub fn check() -> WebViewRuntimeInfo {
    #[cfg(target_os = "windows")]
    {
        let version = match read_installed_version() {
            Ok(version) => version,
            Err(failure) => {
                return WebViewRuntimeInfo {
                    version: None,
                    supported: false,
                    failure: Some(failure),
                }
            }
        };
        match classify_version(&version) {
            Ok(_) => {}
            Err(failure) => {
                return WebViewRuntimeInfo {
                    version: Some(version),
                    supported: false,
                    failure: Some(failure),
                }
            }
        };
        return WebViewRuntimeInfo {
            version: Some(version),
            supported: true,
            failure: None,
        };
    }

    // macOS/Linux 的 WebKit 由系统提供，当前没有引入额外的 GTK/Objective-C 绑定；
    // 不把 Windows WebView2 API 误用于其它平台，也不因此阻断尚未实现门禁的平台。
    #[cfg(not(target_os = "windows"))]
    WebViewRuntimeInfo {
        version: None,
        supported: true,
        failure: None,
    }
}

#[cfg(target_os = "windows")]
fn read_installed_version() -> Result<String, WebViewRuntimeFailure> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Com::CoTaskMemFree;

    let mut version_ptr = PWSTR::null();
    let result = unsafe {
        webview2_com::Microsoft::Web::WebView2::Win32::GetAvailableCoreWebView2BrowserVersionString(
            PCWSTR::null(),
            &mut version_ptr,
        )
    };
    if let Err(error) = result {
        if !version_ptr.is_null() {
            unsafe { CoTaskMemFree(Some(version_ptr.0.cast())) };
        }
        return Err(if is_runtime_missing_error(error.code().0) {
            WebViewRuntimeFailure::Missing
        } else {
            WebViewRuntimeFailure::VersionCheckFailed
        });
    }
    if version_ptr.is_null() {
        return Err(WebViewRuntimeFailure::Missing);
    }

    let value = match unsafe { version_ptr.to_string() } {
        Ok(value) => value,
        Err(_) => {
            unsafe { CoTaskMemFree(Some(version_ptr.0.cast())) };
            return Err(WebViewRuntimeFailure::VersionInvalid);
        }
    };
    unsafe { CoTaskMemFree(Some(version_ptr.0.cast())) };
    Ok(value)
}

#[cfg(target_os = "windows")]
fn is_runtime_missing_error(code: i32) -> bool {
    matches!(code as u32, 0x8007_0002 | 0x8007_0003)
}

fn parse_version(value: &str) -> Option<[u32; 4]> {
    let parts = value.trim().split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    let mut parsed = [0; 4];
    for (index, part) in parts.into_iter().enumerate() {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        parsed[index] = part.parse().ok()?;
    }
    Some(parsed)
}

fn classify_version(value: &str) -> Result<[u32; 4], WebViewRuntimeFailure> {
    let parsed = parse_version(value).ok_or(WebViewRuntimeFailure::VersionInvalid)?;
    if parsed.cmp(&MINIMUM_WEBVIEW2_VERSION) == Ordering::Less {
        return Err(WebViewRuntimeFailure::VersionTooOld);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{classify_version, parse_version, WebViewRuntimeFailure, MINIMUM_WEBVIEW2_VERSION};

    #[test]
    fn accepts_m111_minimum_version() {
        assert!(parse_version("111.0.1661.54").unwrap() >= MINIMUM_WEBVIEW2_VERSION);
    }

    #[test]
    fn rejects_older_or_malformed_versions() {
        assert!(parse_version("110.0.1587.57").unwrap() < MINIMUM_WEBVIEW2_VERSION);
        assert_eq!(parse_version("111.0.1661"), None);
    }

    #[test]
    fn classifies_supported_old_and_invalid_versions() {
        assert!(classify_version("111.0.1661.54").is_ok());
        assert_eq!(
            classify_version("110.0.1587.57"),
            Err(WebViewRuntimeFailure::VersionTooOld)
        );
        assert_eq!(
            classify_version("not-a-version"),
            Err(WebViewRuntimeFailure::VersionInvalid)
        );
    }

    #[test]
    fn exposes_stable_webview_failure_codes() {
        assert_eq!(WebViewRuntimeFailure::Missing.code(), "webview2_missing");
        assert_eq!(
            WebViewRuntimeFailure::VersionTooOld.code(),
            "webview2_version_too_old"
        );
        assert_eq!(
            WebViewRuntimeFailure::VersionInvalid.code(),
            "webview2_version_invalid"
        );
        assert_eq!(
            WebViewRuntimeFailure::VersionCheckFailed.code(),
            "webview2_version_check_failed"
        );
    }

    #[test]
    fn exposes_user_diagnostic_codes() {
        assert_eq!(
            WebViewRuntimeFailure::Missing.diagnostic_code(),
            "WEBVIEW2_MISSING"
        );
        assert_eq!(
            WebViewRuntimeFailure::VersionTooOld.diagnostic_code(),
            "WEBVIEW2_VERSION_TOO_OLD"
        );
        assert_eq!(
            WebViewRuntimeFailure::VersionInvalid.diagnostic_code(),
            "WEBVIEW2_VERSION_INVALID"
        );
        assert_eq!(
            WebViewRuntimeFailure::VersionCheckFailed.diagnostic_code(),
            "WEBVIEW2_VERSION_CHECK_FAILED"
        );
        assert_eq!(
            WebViewRuntimeFailure::ForcedBlock.diagnostic_code(),
            "WEBVIEW2_FORCED_BLOCK"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn recognizes_missing_runtime_hresult_values() {
        assert!(super::is_runtime_missing_error(0x8007_0002_u32 as i32));
        assert!(super::is_runtime_missing_error(0x8007_0003_u32 as i32));
        assert!(!super::is_runtime_missing_error(0x8000_4005_u32 as i32));
    }
}
