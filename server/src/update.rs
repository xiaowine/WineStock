//! 无头 Server Shell 的统一发布清单检查。
//!
//! 本模块只检查统一清单并输出下载位置；Server 进程不在运行中替换自身文件，安装仍由部署流程负责。

use serde::Deserialize;
use url::Url;

const UPDATE_MANIFEST_URL: &str = "https://api.ikuns.top/WineRealm/file/winestock/winestock.json";

/// Server Shell 的更新检查结果。
#[derive(Debug, Clone)]
pub struct ServerUpdateCheckResult {
    /// 当前 Server Shell 的共享发行版本。
    pub current_version: String,
    /// 清单声明的最新统一发行版本。
    pub latest_version: String,
    /// Server 制品的完整下载地址。
    pub download_url: String,
    /// 是否存在比当前版本更高的发行版本。
    pub update_available: bool,
}

/// Server Shell 更新检查失败。
#[derive(Debug)]
pub struct ServerUpdateError(String);

impl std::fmt::Display for ServerUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ServerUpdateError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateManifest {
    version: String,
    #[serde(rename = "baseUrl")]
    base_url: String,
    server: UpdateAsset,
    #[serde(default)]
    notes: String,
    desktop: UpdateAsset,
    android: UpdateAsset,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateAsset {
    file: String,
    sha256: String,
}

/// 请求与 Desktop、Android 相同的发布清单，供部署自动化显式检查版本。
pub async fn check_for_update() -> Result<ServerUpdateCheckResult, ServerUpdateError> {
    let current_version = env!("CARGO_PKG_VERSION").to_owned();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("WineStock Server/{current_version}"))
        .build()
        .map_err(|_| ServerUpdateError("更新服务暂时不可用".to_owned()))?;
    let response = client
        .get(UPDATE_MANIFEST_URL)
        .send()
        .await
        .map_err(|_| ServerUpdateError("暂时无法连接更新服务".to_owned()))?;
    if !response.status().is_success() {
        return Err(ServerUpdateError("更新服务暂时不可用".to_owned()));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.starts_with("application/json") && !content_type.contains("+json") {
        return Err(ServerUpdateError("更新清单格式无效".to_owned()));
    }
    let manifest = response
        .json::<UpdateManifest>()
        .await
        .map_err(|_| ServerUpdateError("更新清单格式无效".to_owned()))?;
    let download_url = validate_manifest(&manifest)?;
    let update_available = compare_versions(&manifest.version, &current_version)? > 0;
    Ok(ServerUpdateCheckResult {
        current_version,
        latest_version: manifest.version,
        download_url,
        update_available,
    })
}

fn validate_manifest(manifest: &UpdateManifest) -> Result<String, ServerUpdateError> {
    let _ = (&manifest.notes, &manifest.desktop, &manifest.android);
    if parse_version(&manifest.version).is_none() {
        return Err(ServerUpdateError("更新版本格式无效".to_owned()));
    }
    if manifest.server.sha256.len() != 64
        || !manifest.server.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ServerUpdateError("更新摘要格式无效".to_owned()));
    }
    let base = Url::parse(&manifest.base_url)
        .map_err(|_| ServerUpdateError("更新基础地址无效".to_owned()))?;
    if base.scheme() != "https"
        || base.username() != ""
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(ServerUpdateError("更新基础地址必须使用不含凭据的 HTTPS".to_owned()));
    }
    let file = manifest.server.file.trim();
    if file.is_empty()
        || !file.to_ascii_lowercase().ends_with(".zip")
        || file.starts_with('/')
        || file.contains('?')
        || file.contains('#')
        || file.split('/').any(|segment| matches!(segment, "." | ".."))
    {
        return Err(ServerUpdateError("Server 更新文件名无效".to_owned()));
    }
    Url::parse(&format!("{}/", manifest.base_url.trim_end_matches('/')))
        .and_then(|base| base.join(file))
        .map(|url| url.into())
        .map_err(|_| ServerUpdateError("更新文件地址无效".to_owned()))
}

fn compare_versions(left: &str, right: &str) -> Result<i8, ServerUpdateError> {
    let left = parse_version(left).ok_or_else(|| ServerUpdateError("更新版本格式无效".to_owned()))?;
    let right = parse_version(right).ok_or_else(|| ServerUpdateError("当前版本格式无效".to_owned()))?;
    Ok(match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

fn parse_version(value: &str) -> Option<[u64; 3]> {
    let core = value.trim().split_once('-').map_or(value.trim(), |(core, _)| core);
    let parts: Vec<u64> = core.split('.').map(str::parse).collect::<Result<_, _>>().ok()?;
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut normalized = [0; 3];
    normalized[..parts.len()].copy_from_slice(&parts);
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, parse_version};

    #[test]
    fn compares_semantic_versions_numerically() {
        assert_eq!(compare_versions("0.1.10", "0.1.9").unwrap(), 1);
        assert_eq!(compare_versions("0.1.0", "0.1.0").unwrap(), 0);
    }

    #[test]
    fn rejects_invalid_versions() {
        assert!(parse_version("development").is_none());
    }
}
