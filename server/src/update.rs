//! 无头 Server Shell 的 GitHub Release 更新检查。
//!
//! Server 只读取最新 Release 的版本信息，不下载或安装更新。

use serde::Deserialize;

const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/xiaowine/WineStock/releases/latest";

/// Server Shell 的更新检查结果。
#[derive(Debug, Clone)]
pub struct ServerUpdateCheckResult {
    /// 当前 Server Shell 的共享发行版本。
    pub current_version: String,
    /// GitHub Release 声明的最新发行版本。
    pub latest_version: String,
    /// GitHub Release 页面地址。
    pub release_url: String,
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
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

/// 请求 GitHub 最新正式 Release，供部署自动化显式检查版本。
pub async fn check_for_update() -> Result<ServerUpdateCheckResult, ServerUpdateError> {
    let current_version = env!("CARGO_PKG_VERSION").to_owned();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("WineStock Server/{current_version}"))
        .build()
        .map_err(|_| ServerUpdateError("更新服务暂时不可用".to_owned()))?;
    let response = client
        .get(GITHUB_RELEASES_URL)
        .send()
        .await
        .map_err(|_| ServerUpdateError("暂时无法连接更新服务".to_owned()))?;
    if !response.status().is_success() {
        return Err(ServerUpdateError("GitHub 更新服务暂时不可用".to_owned()));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.starts_with("application/json") && !content_type.contains("+json") {
        return Err(ServerUpdateError("GitHub 更新信息格式无效".to_owned()));
    }
    let release = response
        .json::<GithubRelease>()
        .await
        .map_err(|_| ServerUpdateError("GitHub 更新信息格式无效".to_owned()))?;
    if release.draft || release.prerelease {
        return Err(ServerUpdateError(
            "GitHub 最新 Release 不是正式版本".to_owned(),
        ));
    }
    let latest_version = release.tag_name.trim().trim_start_matches('v').to_owned();
    if parse_version(&latest_version).is_none()
        || !release.html_url.starts_with("https://github.com/")
    {
        return Err(ServerUpdateError("GitHub 更新信息格式无效".to_owned()));
    }
    let update_available = compare_versions(&latest_version, &current_version)? > 0;
    Ok(ServerUpdateCheckResult {
        current_version,
        latest_version,
        release_url: release.html_url,
        update_available,
    })
}

fn compare_versions(left: &str, right: &str) -> Result<i8, ServerUpdateError> {
    let left =
        parse_version(left).ok_or_else(|| ServerUpdateError("更新版本格式无效".to_owned()))?;
    let right =
        parse_version(right).ok_or_else(|| ServerUpdateError("当前版本格式无效".to_owned()))?;
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
