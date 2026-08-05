//! Desktop 更新检查、安装包下载和 Windows 安装器启动。
//!
//! 本模块属于 Desktop Shell，只访问固定的公开更新清单，不代理 core HTTP，也不把下载地址或
//! 临时文件路径交给前端。当前 Desktop 发布面使用 Windows 安装器；其它桌面系统明确返回不支持。

use std::{fs, process::Command, time::Duration};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use url::Url;

const UPDATE_MANIFEST_URL: &str = "https://api.ikuns.top/WineRealm/file/winestock/desktop.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_UPDATE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateManifest {
    version: String,
    url: String,
    sha256: String,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateCheckResult {
    /// 当前 Desktop Shell 的包版本。
    pub current_version: String,
    /// 只有发现更高版本时才返回；没有更新时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// 更新说明；只有发现更新时返回。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct UpdateError {
    pub code: &'static str,
    pub message: String,
}

impl UpdateError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// 请求 Desktop 清单并返回当前版本是否有可用更新。
pub async fn check_for_update(app: &AppHandle) -> Result<AppUpdateCheckResult, UpdateError> {
    let current_version = app.package_info().version.to_string();
    let manifest = fetch_manifest(&current_version).await?;
    let has_update = compare_versions(&manifest.version, &current_version)? > 0;
    Ok(result_from_manifest(manifest, has_update, &current_version))
}

/// 重新检查指定版本并下载、启动 Windows 安装器。
pub async fn install_update(app: &AppHandle, expected_version: &str) -> Result<(), UpdateError> {
    let current_version = app.package_info().version.to_string();
    let manifest = fetch_manifest(&current_version).await?;
    let comparison = compare_versions(&manifest.version, &current_version)?;
    if comparison <= 0 || manifest.version != expected_version {
        return Err(UpdateError::new(
            "update_not_available",
            "请求安装的版本已经不可用，请重新检查更新",
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        return Err(UpdateError::new(
            "update_install_failed",
            "当前桌面平台没有可用的 WineStock 安装器",
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let bytes = download_update(&manifest, &current_version).await?;
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(|_| UpdateError::new("update_download_failed", "无法准备更新缓存"))?;
        fs::create_dir_all(&cache_dir)
            .map_err(|_| UpdateError::new("update_download_failed", "无法准备更新缓存"))?;
        let installer_path = cache_dir.join(format!("winestock-update-{}.exe", manifest.version));
        fs::write(&installer_path, bytes)
            .map_err(|_| UpdateError::new("update_download_failed", "无法保存更新安装器"))?;

        Command::new(&installer_path)
            .spawn()
            .map_err(|_| UpdateError::new("update_install_failed", "无法启动更新安装器"))?;
        // 安装器接管后由 Desktop 生命周期统一停止本地 core，避免覆盖运行中的文件。
        app.exit(0);
        Ok(())
    }
}

async fn fetch_manifest(current_version: &str) -> Result<UpdateManifest, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(format!("WineStock Desktop/{current_version}"))
        .build()
        .map_err(|_| UpdateError::new("update_check_unavailable", "更新服务暂时不可用"))?;
    let response = client
        .get(UPDATE_MANIFEST_URL)
        .send()
        .await
        .map_err(|_| UpdateError::new("update_check_unavailable", "暂时无法连接更新服务"))?;
    if !response.status().is_success() {
        return Err(UpdateError::new(
            "update_check_unavailable",
            "更新服务暂时不可用",
        ));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.starts_with("application/json") && !content_type.contains("+json") {
        return Err(UpdateError::new(
            "update_manifest_invalid",
            "更新服务返回了无效内容类型",
        ));
    }
    let manifest = response
        .json::<UpdateManifest>()
        .await
        .map_err(|_| UpdateError::new("update_manifest_invalid", "更新清单格式无效"))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

async fn download_update(
    manifest: &UpdateManifest,
    current_version: &str,
) -> Result<Vec<u8>, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(format!("WineStock Desktop/{current_version}"))
        .build()
        .map_err(|_| UpdateError::new("update_download_failed", "更新安装器下载失败"))?;
    let response = client
        .get(&manifest.url)
        .send()
        .await
        .map_err(|_| UpdateError::new("update_download_failed", "更新安装器下载失败"))?;
    if !response.status().is_success() {
        return Err(UpdateError::new(
            "update_download_failed",
            "更新安装器下载失败",
        ));
    }
    if response.content_length().unwrap_or(0) > MAX_UPDATE_BYTES {
        return Err(UpdateError::new(
            "update_download_failed",
            "更新安装器超过允许大小",
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| UpdateError::new("update_download_failed", "更新安装器下载失败"))?;
    if bytes.len() as u64 > MAX_UPDATE_BYTES {
        return Err(UpdateError::new(
            "update_download_failed",
            "更新安装器超过允许大小",
        ));
    }
    let actual = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != manifest.sha256 {
        return Err(UpdateError::new(
            "update_integrity_failed",
            "更新安装器校验失败",
        ));
    }
    Ok(bytes.to_vec())
}

fn validate_manifest(manifest: &UpdateManifest) -> Result<(), UpdateError> {
    if parse_version(&manifest.version).is_none() {
        return Err(UpdateError::new(
            "update_manifest_invalid",
            "更新版本格式无效",
        ));
    }
    let url = Url::parse(&manifest.url)
        .map_err(|_| UpdateError::new("update_manifest_invalid", "更新安装器地址无效"))?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err(UpdateError::new(
            "update_manifest_invalid",
            "更新安装器地址必须使用不含凭据的 HTTPS",
        ));
    }
    if !url.path().to_ascii_lowercase().ends_with(".exe") {
        return Err(UpdateError::new(
            "update_manifest_invalid",
            "Desktop 更新地址不是 Windows 安装器",
        ));
    }
    if manifest.sha256.len() != 64 || !manifest.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(UpdateError::new(
            "update_manifest_invalid",
            "更新摘要格式无效",
        ));
    }
    Ok(())
}

fn result_from_manifest(
    manifest: UpdateManifest,
    has_update: bool,
    current_version: &str,
) -> AppUpdateCheckResult {
    if has_update {
        AppUpdateCheckResult {
            current_version: current_version.to_owned(),
            latest_version: Some(manifest.version),
            notes: (!manifest.notes.trim().is_empty()).then_some(manifest.notes),
        }
    } else {
        AppUpdateCheckResult {
            current_version: current_version.to_owned(),
            latest_version: None,
            notes: None,
        }
    }
}

fn compare_versions(left: &str, right: &str) -> Result<i8, UpdateError> {
    let left = parse_version(left)
        .ok_or_else(|| UpdateError::new("update_manifest_invalid", "更新版本格式无效"))?;
    let right = parse_version(right)
        .ok_or_else(|| UpdateError::new("update_manifest_invalid", "当前版本格式无效"))?;
    Ok(match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

fn parse_version(value: &str) -> Option<[u64; 3]> {
    let core = value
        .trim()
        .split_once('-')
        .map_or(value.trim(), |(core, _)| core);
    let parts: Vec<u64> = core
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut normalized = [0; 3];
    normalized[..parts.len()].copy_from_slice(&parts);
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::{
        compare_versions, parse_version, result_from_manifest, validate_manifest, UpdateManifest,
    };

    #[test]
    fn compares_semantic_versions_numerically() {
        assert_eq!(compare_versions("0.1.10", "0.1.9").unwrap(), 1);
        assert_eq!(compare_versions("0.1.0", "0.1.0").unwrap(), 0);
        assert_eq!(compare_versions("0.0.9", "0.1.0").unwrap(), -1);
    }

    #[test]
    fn rejects_invalid_versions() {
        assert!(parse_version("development").is_none());
        assert!(parse_version("1.2.3.4").is_none());
    }

    #[test]
    fn rejects_non_windows_update_assets() {
        let manifest = UpdateManifest {
            version: "0.1.1".to_owned(),
            url: "https://download.example.com/WineStock-0.1.1.apk".to_owned(),
            sha256: "a".repeat(64),
            notes: String::new(),
        };
        let error = validate_manifest(&manifest).unwrap_err();
        assert_eq!(error.code, "update_manifest_invalid");
    }

    #[test]
    fn accepts_valid_windows_update_manifest() {
        let manifest = UpdateManifest {
            version: "0.1.1".to_owned(),
            url: "https://download.example.com/WineStock-0.1.1-setup.exe".to_owned(),
            sha256: "a".repeat(64),
            notes: String::new(),
        };
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn keeps_runtime_current_version_separate_from_manifest_version() {
        let manifest = UpdateManifest {
            version: "0.1.0".to_owned(),
            url: "https://download.example.com/WineStock-0.1.0-setup.exe".to_owned(),
            sha256: "a".repeat(64),
            notes: String::new(),
        };
        let result = result_from_manifest(manifest, true, "0.0.1");

        assert_eq!(result.current_version, "0.0.1");
        assert_eq!(result.latest_version.as_deref(), Some("0.1.0"));
    }
}
