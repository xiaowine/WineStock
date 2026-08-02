//! Tauri command 层：把 Shell Bridge v1 方法映射到 DesktopRuntimeManager。
//!
//! 本模块只做参数校验、状态查询、首屏就绪窗口协作与错误序列化，不包含业务逻辑。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    contract::{ApplyRuntimeConfigResult, EditableRuntimeConfig, RuntimeConfigValidationResult},
    preferences::{DesktopPreferences, DesktopPreferencesState},
    runtime::DesktopRuntimeManager,
};

type CommandResult<T> = Result<T, String>;

static FRONTEND_READY: AtomicBool = AtomicBool::new(false);
static FRONTEND_FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

/// 返回当前进程是否已收到前端首帧就绪信号；仅供 Desktop 窗口显示兜底使用。
pub fn is_frontend_ready() -> bool {
    FRONTEND_READY.load(Ordering::Acquire)
}

/// 返回前端是否已经上报启动失败；超时兜底不能覆盖原生失败提示。
pub fn is_frontend_failure_reported() -> bool {
    FRONTEND_FAILURE_REPORTED.load(Ordering::Acquire)
}

fn command_error(code: &str, message: &str) -> String {
    serde_json::json!({ "code": code, "message": message }).to_string()
}

#[tauri::command]
pub async fn shell_get_runtime_snapshot(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
    debug: State<'_, crate::lifecycle::DebugStartupOverrides>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
    if debug.force_shell_bridge_block {
        return Err(command_error(
            "invalid_bridge_payload",
            "测试：Shell Bridge 初始化失败",
        ));
    }
    Ok(manager.snapshot().await)
}

#[tauri::command]
pub async fn shell_validate_runtime_config(
    config: EditableRuntimeConfig,
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<RuntimeConfigValidationResult> {
    Ok(manager.validate(config).await)
}

#[tauri::command]
pub async fn shell_apply_runtime_config(
    config: EditableRuntimeConfig,
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<ApplyRuntimeConfigResult> {
    Ok(manager.apply(config).await)
}

#[tauri::command]
pub async fn shell_start_local_service(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
    manager
        .start_local_service()
        .await
        .map_err(|error| command_error(&error.code, &error.message))
}

#[tauri::command]
pub async fn shell_stop_local_service(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
    manager
        .stop_local_service()
        .await
        .map_err(|error| command_error(&error.code, &error.message))
}

#[tauri::command]
pub async fn shell_restart_local_service(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
    manager
        .restart_local_service()
        .await
        .map_err(|error| command_error(&error.code, &error.message))
}

#[tauri::command]
pub async fn shell_repair_firewall(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
    manager
        .repair_firewall()
        .await
        .map_err(|error| command_error(&error.code, &error.message))
}

#[tauri::command]
pub fn shell_get_desktop_preferences(
    app: AppHandle,
    preferences: State<'_, DesktopPreferencesState>,
) -> CommandResult<DesktopPreferences> {
    let mut value = preferences.get();
    value.autostart_enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| command_error("desktop_autostart_unavailable", &error.to_string()))?;
    Ok(value)
}

#[tauri::command]
pub fn shell_set_desktop_preferences(
    app: AppHandle,
    preferences: DesktopPreferences,
    state: State<'_, DesktopPreferencesState>,
) -> CommandResult<DesktopPreferences> {
    let autostart = app.autolaunch();
    let previous_autostart = autostart
        .is_enabled()
        .map_err(|error| command_error("desktop_autostart_unavailable", &error.to_string()))?;
    let autostart_changed = previous_autostart != preferences.autostart_enabled;

    if autostart_changed {
        let result = if preferences.autostart_enabled {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(error) = result {
            return Err(command_error(
                "desktop_autostart_unavailable",
                &format!("无法更新开机自启设置：{error}"),
            ));
        }
    }

    let saved = match state.set(preferences) {
        Ok(value) => value,
        Err(error) => {
            if autostart_changed {
                let rollback = if previous_autostart {
                    autostart.enable()
                } else {
                    autostart.disable()
                };
                if let Err(rollback_error) = rollback {
                    eprintln!("WineStock 开机自启状态回滚失败：{rollback_error}");
                }
            }
            return Err(command_error("desktop_preferences_unavailable", &error));
        }
    };

    let mut result = saved;
    result.autostart_enabled = autostart
        .is_enabled()
        .map_err(|error| command_error("desktop_autostart_unavailable", &error.to_string()))?;
    Ok(result)
}

#[tauri::command]
pub async fn shell_frontend_ready(
    app: AppHandle,
    debug: State<'_, crate::lifecycle::DebugStartupOverrides>,
) -> CommandResult<()> {
    if debug.force_shell_bridge_handshake_block {
        return Err(command_error(
            "invalid_bridge_payload",
            "测试：Shell Bridge 首屏握手失败",
        ));
    }
    // 只有前端完成首帧渲染后才显示主窗口，避免 WebView 加载期间出现白屏或闪烁。
    FRONTEND_READY.store(true, Ordering::Release);
    if !app
        .state::<crate::lifecycle::AppLifecycleState>()
        .startup_silent()
        || !app
            .state::<crate::lifecycle::AppLifecycleState>()
            .tray_available()
    {
        if let Some(window) = app.get_webview_window("main") {
            crate::window::show_main_window(&window);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn shell_frontend_failed(app: AppHandle, code: String) -> CommandResult<()> {
    if FRONTEND_FAILURE_REPORTED.swap(true, Ordering::AcqRel) {
        return Ok(());
    }

    let reason = normalize_frontend_failure_code(&code);
    let (title, description) = frontend_failure_message(reason);
    let description = append_diagnostic_code(description, frontend_failure_diagnostic_code(reason));
    eprintln!(
        "WineStock Desktop 启动门卫失败：gate=shell_bridge reason={reason} debug={}",
        cfg!(debug_assertions)
    );
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
    app.exit(1);
    Ok(())
}

/// 前端没有上报失败且超过启动窗口时使用的原生兜底。
pub fn show_frontend_load_timeout(app: &AppHandle) {
    if is_frontend_ready() || FRONTEND_FAILURE_REPORTED.swap(true, Ordering::AcqRel) {
        return;
    }

    eprintln!(
        "WineStock Desktop 启动门卫失败：gate=shell_bridge reason=frontend_load_timeout debug={}",
        cfg!(debug_assertions)
    );
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let description = append_diagnostic_code(
        "页面未能在规定时间内完成加载。请重新启动 WineStock；问题仍存在时请重新安装软件。确认后 WineStock 将退出。",
        "FRONTEND_LOAD_TIMEOUT",
    );
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("WineStock 页面加载超时")
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
    app.exit(1);
}

fn append_diagnostic_code(description: &str, diagnostic_code: &str) -> String {
    format!("{description}\n错误代码：{diagnostic_code}")
}

fn frontend_failure_diagnostic_code(code: &str) -> &'static str {
    match code {
        "shell_bridge_unavailable" => "SHELL_BRIDGE_UNAVAILABLE",
        "shell_bridge_snapshot_invalid" => "SHELL_BRIDGE_SNAPSHOT_INVALID",
        "shell_bridge_version_mismatch" => "SHELL_BRIDGE_VERSION_MISMATCH",
        "shell_bridge_method_missing" => "SHELL_BRIDGE_METHOD_MISSING",
        "shell_bridge_extension_invalid" => "SHELL_BRIDGE_EXTENSION_INVALID",
        "shell_bridge_event_subscription_failed" => "SHELL_BRIDGE_EVENT_SUBSCRIPTION_FAILED",
        "shell_bridge_ready_failed" => "SHELL_BRIDGE_READY_FAILED",
        "frontend_load_timeout" => "FRONTEND_LOAD_TIMEOUT",
        _ => "SHELL_BRIDGE_UNAVAILABLE",
    }
}

fn frontend_failure_message(code: &str) -> (&'static str, &'static str) {
    match code {
        "frontend_load_timeout" => (
            "WineStock 页面加载超时",
            "页面未能在规定时间内完成加载。请重新启动 WineStock；问题仍存在时请重新安装软件。确认后 WineStock 将退出。",
        ),
        "shell_bridge_snapshot_invalid" | "shell_bridge_version_mismatch" => (
            "WineStock 无法加载",
            "当前界面与桌面运行组件版本不匹配。请重新安装同一版本的 WineStock。确认后 WineStock 将退出。",
        ),
        "shell_bridge_method_missing" | "shell_bridge_extension_invalid" => (
            "WineStock 无法加载",
            "桌面运行组件缺少必要能力。请重新安装 WineStock。确认后 WineStock 将退出。",
        ),
        "shell_bridge_event_subscription_failed" | "shell_bridge_ready_failed" => (
            "WineStock 页面加载失败",
            "WineStock 页面无法完成启动握手。请重新启动软件；问题仍存在时请重新安装软件。确认后 WineStock 将退出。",
        ),
        _ => (
            "WineStock 无法连接桌面组件",
            "桌面运行组件没有正常响应。请重新启动 WineStock；问题仍存在时请修复或重新安装软件。确认后 WineStock 将退出。",
        ),
    }
}

fn normalize_frontend_failure_code(code: &str) -> &'static str {
    match code {
        "shell_bridge_unavailable" => "shell_bridge_unavailable",
        "shell_bridge_snapshot_invalid" => "shell_bridge_snapshot_invalid",
        "shell_bridge_version_mismatch" => "shell_bridge_version_mismatch",
        "shell_bridge_method_missing" => "shell_bridge_method_missing",
        "shell_bridge_extension_invalid" => "shell_bridge_extension_invalid",
        "shell_bridge_event_subscription_failed" => "shell_bridge_event_subscription_failed",
        "shell_bridge_ready_failed" => "shell_bridge_ready_failed",
        "frontend_load_timeout" => "frontend_load_timeout",
        _ => "shell_bridge_unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        frontend_failure_diagnostic_code, frontend_failure_message, normalize_frontend_failure_code,
    };

    #[test]
    fn keeps_known_failure_codes_and_sanitizes_unknown_values() {
        assert_eq!(
            normalize_frontend_failure_code("shell_bridge_version_mismatch"),
            "shell_bridge_version_mismatch"
        );
        assert_eq!(
            normalize_frontend_failure_code("arbitrary-user-input"),
            "shell_bridge_unavailable"
        );
        assert_eq!(
            frontend_failure_diagnostic_code("arbitrary-user-input"),
            "SHELL_BRIDGE_UNAVAILABLE"
        );
    }

    #[test]
    fn maps_startup_failure_codes_to_actionable_exit_messages() {
        let (title, description) = frontend_failure_message("shell_bridge_ready_failed");
        assert_eq!(title, "WineStock 页面加载失败");
        assert!(description.contains("重新启动"));
        assert!(description.contains("将退出"));

        let (title, description) = frontend_failure_message("shell_bridge_version_mismatch");
        assert_eq!(title, "WineStock 无法加载");
        assert!(description.contains("重新安装同一版本"));
        assert!(description.contains("将退出"));

        let (title, description) = frontend_failure_message("frontend_load_timeout");
        assert_eq!(title, "WineStock 页面加载超时");
        assert!(description.contains("重新启动"));
        assert_eq!(
            frontend_failure_diagnostic_code("shell_bridge_ready_failed"),
            "SHELL_BRIDGE_READY_FAILED"
        );
    }
}
