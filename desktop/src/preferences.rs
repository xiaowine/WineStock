//! Desktop 本机偏好设置的持久化与进程内状态。
//!
//! 本模块只拥有窗口和自启动等 desktop shell 偏好，不参与共享运行配置、业务数据或认证状态。

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{RwLock, RwLockReadGuard},
};

use serde::{Deserialize, Serialize};

pub const DESKTOP_PREFERENCES_VERSION: u32 = 1;
pub const WEBVIEW_RECLAIM_IDLE_MINUTES: [u32; 6] = [5, 15, 30, 60, 120, 240];

fn default_autostart_silent() -> bool {
    true
}

fn default_webview_reclaim_idle_minutes() -> u32 {
    30
}

/// 主窗口收到系统关闭请求时采用的行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloseBehavior {
    MinimizeToTray,
    ExitApplication,
}

impl Default for CloseBehavior {
    fn default() -> Self {
        Self::MinimizeToTray
    }
}

/// 可由前端读取和更新的 desktop 偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopPreferences {
    pub version: u32,
    pub close_behavior: CloseBehavior,
    #[serde(default)]
    pub autostart_enabled: bool,
    #[serde(default = "default_autostart_silent")]
    pub autostart_silent: bool,
    #[serde(default)]
    pub webview_reclaim_enabled: bool,
    #[serde(default = "default_webview_reclaim_idle_minutes")]
    pub webview_reclaim_idle_minutes: u32,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            version: DESKTOP_PREFERENCES_VERSION,
            close_behavior: CloseBehavior::default(),
            autostart_enabled: false,
            autostart_silent: true,
            webview_reclaim_enabled: false,
            webview_reclaim_idle_minutes: default_webview_reclaim_idle_minutes(),
        }
    }
}

/// 校验 Desktop 偏好，确保窗口生命周期使用有限且可预期的等待时间。
pub fn validate_desktop_preferences(preferences: DesktopPreferences) -> Result<(), String> {
    if !WEBVIEW_RECLAIM_IDLE_MINUTES.contains(&preferences.webview_reclaim_idle_minutes) {
        return Err("WebView 回收等待时间不受支持".to_owned());
    }
    Ok(())
}

#[derive(Debug)]
pub struct DesktopPreferencesState {
    path: PathBuf,
    value: RwLock<DesktopPreferences>,
}

impl DesktopPreferencesState {
    pub fn load(path: PathBuf) -> Self {
        let value = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<DesktopPreferences>(&raw) {
                Ok(preferences)
                    if preferences.version == DESKTOP_PREFERENCES_VERSION
                        && validate_desktop_preferences(preferences).is_ok() =>
                {
                    preferences
                }
                _ => {
                    eprintln!("WineStock desktop 偏好格式无效，使用默认偏好");
                    DesktopPreferences::default()
                }
            },
            Err(error) if error.kind() == ErrorKind::NotFound => DesktopPreferences::default(),
            Err(error) => {
                eprintln!("WineStock desktop 偏好读取失败：{error}");
                DesktopPreferences::default()
            }
        };

        Self {
            path,
            value: RwLock::new(value),
        }
    }

    pub fn get(&self) -> DesktopPreferences {
        self.read()
            .map(|preferences| *preferences)
            .unwrap_or_default()
    }

    pub fn set(&self, preferences: DesktopPreferences) -> Result<DesktopPreferences, String> {
        if preferences.version != DESKTOP_PREFERENCES_VERSION {
            return Err("不支持的 desktop 偏好版本".to_owned());
        }
        validate_desktop_preferences(preferences)?;

        let mut current = self
            .value
            .write()
            .map_err(|_| "desktop 偏好状态不可用".to_owned())?;
        let content = serde_json::to_vec_pretty(&preferences)
            .map_err(|_| "desktop 偏好序列化失败".to_owned())?;
        atomic_write(&self.path, &content).map_err(|_| "无法保存 desktop 偏好".to_owned())?;
        *current = preferences;
        Ok(preferences)
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, DesktopPreferences>, ()> {
        self.value.read().map_err(|_| ())
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    fs::write(temporary.path(), content)?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_minimize_to_tray() {
        let defaults = DesktopPreferences::default();
        assert_eq!(defaults.close_behavior, CloseBehavior::MinimizeToTray);
        assert!(!defaults.autostart_enabled);
        assert!(defaults.autostart_silent);
        assert!(!defaults.webview_reclaim_enabled);
        assert_eq!(defaults.webview_reclaim_idle_minutes, 30);
    }

    #[test]
    fn persists_and_loads_preferences() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("desktop-preferences.json");
        let state = DesktopPreferencesState::load(path.clone());
        let preferences = DesktopPreferences {
            version: DESKTOP_PREFERENCES_VERSION,
            close_behavior: CloseBehavior::ExitApplication,
            autostart_enabled: false,
            autostart_silent: false,
            webview_reclaim_enabled: true,
            webview_reclaim_idle_minutes: 60,
        };

        assert_eq!(state.set(preferences).expect("save"), preferences);
        assert_eq!(DesktopPreferencesState::load(path).get(), preferences);
    }

    #[test]
    fn rejects_unknown_version_without_writing() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("desktop-preferences.json");
        let state = DesktopPreferencesState::load(path.clone());
        let invalid = DesktopPreferences {
            version: DESKTOP_PREFERENCES_VERSION + 1,
            close_behavior: CloseBehavior::ExitApplication,
            autostart_enabled: false,
            autostart_silent: false,
            webview_reclaim_enabled: false,
            webview_reclaim_idle_minutes: 30,
        };

        assert!(state.set(invalid).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn loads_legacy_preferences_with_new_defaults() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("desktop-preferences.json");
        fs::write(&path, r#"{"version":1,"closeBehavior":"exit-application"}"#)
            .expect("write legacy preferences");

        let preferences = DesktopPreferencesState::load(path).get();
        assert_eq!(preferences.close_behavior, CloseBehavior::ExitApplication);
        assert!(!preferences.autostart_enabled);
        assert!(preferences.autostart_silent);
        assert!(!preferences.webview_reclaim_enabled);
        assert_eq!(preferences.webview_reclaim_idle_minutes, 30);
    }

    #[test]
    fn rejects_unsupported_webview_reclaim_idle_minutes() {
        let mut preferences = DesktopPreferences::default();
        preferences.webview_reclaim_idle_minutes = 1;
        assert!(validate_desktop_preferences(preferences).is_err());
        assert!(DesktopPreferencesState::load(
            tempfile::tempdir().unwrap().path().join("prefs.json")
        )
        .set(preferences)
        .is_err());
    }

    #[test]
    fn falls_back_when_persisted_webview_reclaim_idle_minutes_are_invalid() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("desktop-preferences.json");
        fs::write(
            &path,
            r#"{"version":1,"closeBehavior":"minimize-to-tray","webviewReclaimIdleMinutes":1}"#,
        )
        .expect("write invalid preferences");

        let preferences = DesktopPreferencesState::load(path).get();
        assert_eq!(preferences, DesktopPreferences::default());
    }
}
