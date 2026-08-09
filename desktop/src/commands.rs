//! Tauri command 层：把 Shell Bridge v1 方法映射到 DesktopRuntimeManager。
//!
//! 本模块只做参数校验、状态查询、首屏就绪窗口协作与错误序列化，不包含业务逻辑。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    contract::{
        ApplyRuntimeConfigResult, EditableRuntimeConfig, RuntimeConfigValidationResult, WindowTheme,
    },
    native_i18n,
    preferences::{validate_desktop_preferences, DesktopPreferences, DesktopPreferencesState},
    runtime::DesktopRuntimeManager,
};

type CommandResult<T> = Result<T, String>;

static FRONTEND_READY: AtomicBool = AtomicBool::new(false);
static FRONTEND_FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);
static FRONTEND_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 开始一代新的 WebView 前端加载，清理上一代的握手状态。
pub fn begin_frontend_load(generation: u64) {
    FRONTEND_GENERATION.store(generation, Ordering::Release);
    FRONTEND_READY.store(false, Ordering::Release);
    FRONTEND_FAILURE_REPORTED.store(false, Ordering::Release);
}

/// 返回当前进程是否已收到前端首帧就绪信号；仅供 Desktop 窗口显示兜底使用。
pub fn is_frontend_ready() -> bool {
    FRONTEND_READY.load(Ordering::Acquire)
}

fn is_current_frontend_generation(generation: u64) -> bool {
    FRONTEND_GENERATION.load(Ordering::Acquire) == generation
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

/// 检查 Desktop 更新清单；网络和清单解析均留在 Desktop Shell 内。
#[tauri::command]
pub async fn shell_check_for_update(
    app: AppHandle,
) -> CommandResult<crate::update::AppUpdateCheckResult> {
    crate::update::check_for_update(&app)
        .await
        .map_err(|error| command_error(error.code, &error.message))
}

/// 重新确认并启动 Desktop 更新安装器；安装器启动后由应用生命周期负责退出。
#[tauri::command]
pub async fn shell_install_update(app: AppHandle, version: String) -> CommandResult<()> {
    crate::update::install_update(&app, &version)
        .await
        .map_err(|error| command_error(error.code, &error.message))
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
    validate_desktop_preferences(preferences)
        .map_err(|error| command_error("desktop_preferences_invalid", &error))?;
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

    // 偏好可能改变当前排队任务的开关或等待时间；统一重排可见状态下的任务，
    // 关闭回收时也会通过同一入口使旧 token 失效。
    crate::window::schedule_webview_reclaim(&app);

    let mut result = saved;
    result.autostart_enabled = autostart
        .is_enabled()
        .map_err(|error| command_error("desktop_autostart_unavailable", &error.to_string()))?;
    Ok(result)
}

/// 通过 Shell Bridge 同步主窗口外观；只有 Windows 调用 Tauri 原生主题 API。
#[tauri::command]
pub fn shell_set_window_theme(window: WebviewWindow, theme: WindowTheme) -> CommandResult<()> {
    #[cfg(target_os = "windows")]
    {
        let native_theme = match theme {
            WindowTheme::System => None,
            WindowTheme::Light => Some(tauri::Theme::Light),
            WindowTheme::Dark => Some(tauri::Theme::Dark),
        };
        window.set_theme(native_theme).map_err(|error| {
            command_error("desktop_window_theme_unavailable", &error.to_string())
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, theme);
    }

    Ok(())
}

#[tauri::command]
pub async fn shell_frontend_ready(
    app: AppHandle,
    generation: Option<u64>,
    debug: State<'_, crate::lifecycle::DebugStartupOverrides>,
) -> CommandResult<()> {
    if debug.force_shell_bridge_handshake_block {
        return Err(command_error(
            "invalid_bridge_payload",
            "测试：Shell Bridge 首屏握手失败",
        ));
    }
    let lifecycle = app.state::<crate::lifecycle::AppLifecycleState>();
    let current_generation = lifecycle.webview_generation();
    if generation != Some(current_generation) || !is_current_frontend_generation(current_generation)
    {
        return Ok(());
    }

    // 只有前端完成首帧渲染后才显示主窗口，避免 WebView 加载期间出现白屏或闪烁。
    // 若用户在握手完成前已经关闭窗口，则保留该隐藏意图。
    let was_hidden = lifecycle.webview_state() == crate::lifecycle::WebviewState::Hidden;
    FRONTEND_READY.store(true, Ordering::Release);
    lifecycle.mark_webview_ready(current_generation);
    if !was_hidden && (lifecycle.show_webview_on_ready() || !lifecycle.tray_available()) {
        if let Some(window) = app.get_webview_window("main") {
            crate::window::show_main_window(&window);
        }
    } else {
        lifecycle.mark_webview_hidden(current_generation);
        crate::window::schedule_webview_reclaim(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn shell_frontend_failed(
    app: AppHandle,
    code: String,
    generation: Option<u64>,
) -> CommandResult<()> {
    let reason = normalize_frontend_failure_code(&code);
    let lifecycle = app.state::<crate::lifecycle::AppLifecycleState>();
    let current_generation = lifecycle.webview_generation();
    if generation != Some(current_generation) || !is_current_frontend_generation(current_generation)
    {
        return Ok(());
    }
    if FRONTEND_FAILURE_REPORTED.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let locale = native_i18n::system_locale();
    let strings = native_i18n::gate_dialog_strings(locale, frontend_failure_kind(reason));
    let description =
        native_i18n::append_diagnostic_code(locale, strings.description, frontend_failure_diagnostic_code(reason));
    eprintln!(
        "WineStock Desktop 启动门卫失败：gate=shell_bridge reason={reason} debug={}",
        cfg!(debug_assertions)
    );
    if current_generation > 1 {
        crate::window::discard_failed_webview(&app, current_generation);
        return Ok(());
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(strings.title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
    app.exit(1);
    Ok(())
}

/// 前端没有上报失败且超过启动窗口时使用的原生兜底。
pub fn spawn_frontend_load_timeout(app: AppHandle, generation: u64, timeout: std::time::Duration) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(timeout).await;
        show_frontend_load_timeout(&app, generation);
    });
}

pub fn show_frontend_load_timeout(app: &AppHandle, generation: u64) {
    if !is_current_frontend_generation(generation)
        || is_frontend_ready()
        || FRONTEND_FAILURE_REPORTED.swap(true, Ordering::AcqRel)
    {
        return;
    }

    eprintln!(
        "WineStock Desktop 启动门卫失败：gate=shell_bridge reason=frontend_load_timeout debug={}",
        cfg!(debug_assertions)
    );
    if generation > 1 {
        crate::window::discard_failed_webview(app, generation);
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let locale = native_i18n::system_locale();
    let strings =
        native_i18n::gate_dialog_strings(locale, native_i18n::GateFailureKind::FrontendLoadTimeout);
    let description =
        native_i18n::append_diagnostic_code(locale, strings.description, "FRONTEND_LOAD_TIMEOUT");
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(strings.title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
    app.exit(1);
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

/// 把前端失败类别映射为门禁文案种类；文案随系统语言变化，诊断码不变。
fn frontend_failure_kind(code: &str) -> native_i18n::GateFailureKind {
    match code {
        "frontend_load_timeout" => native_i18n::GateFailureKind::FrontendLoadTimeout,
        "shell_bridge_snapshot_invalid" | "shell_bridge_version_mismatch" => {
            native_i18n::GateFailureKind::ShellBridgeMismatch
        }
        "shell_bridge_method_missing" | "shell_bridge_extension_invalid" => {
            native_i18n::GateFailureKind::ShellBridgeMissingCapability
        }
        "shell_bridge_event_subscription_failed" | "shell_bridge_ready_failed" => {
            native_i18n::GateFailureKind::ShellBridgeHandshakeFailed
        }
        _ => native_i18n::GateFailureKind::ShellBridgeUnavailable,
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
        frontend_failure_diagnostic_code, frontend_failure_kind, normalize_frontend_failure_code,
    };
    use crate::native_i18n::{gate_dialog_strings, system_locale, GateFailureKind};

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
    fn maps_startup_failure_codes_to_actionable_exit_dialogs() {
        let locale = system_locale();
        let dialog = gate_dialog_strings(locale, frontend_failure_kind("shell_bridge_ready_failed"));
        assert_eq!(dialog.title, "WineStock 页面加载失败");
        assert!(dialog.description.contains("重新启动"));
        assert!(dialog.description.contains("将退出"));

        let dialog =
            gate_dialog_strings(locale, frontend_failure_kind("shell_bridge_version_mismatch"));
        assert_eq!(dialog.title, "WineStock 无法加载");
        assert!(dialog.description.contains("重新安装同一版本"));
        assert!(dialog.description.contains("将退出"));

        let dialog = gate_dialog_strings(locale, frontend_failure_kind("frontend_load_timeout"));
        assert_eq!(dialog.title, "WineStock 页面加载超时");
        assert!(dialog.description.contains("重新启动"));
        assert_eq!(
            frontend_failure_diagnostic_code("shell_bridge_ready_failed"),
            "SHELL_BRIDGE_READY_FAILED"
        );
    }

    #[test]
    fn every_failure_kind_has_localized_copy_in_both_languages() {
        for kind in [
            GateFailureKind::WebView2Missing,
            GateFailureKind::WebView2VersionTooOld,
            GateFailureKind::WebView2VersionInvalid,
            GateFailureKind::WebView2VersionCheckFailed,
            GateFailureKind::WebView2ForcedBlock,
            GateFailureKind::FrontendLoadTimeout,
            GateFailureKind::ShellBridgeMismatch,
            GateFailureKind::ShellBridgeMissingCapability,
            GateFailureKind::ShellBridgeHandshakeFailed,
            GateFailureKind::ShellBridgeUnavailable,
        ] {
            let zh = gate_dialog_strings(crate::native_i18n::NativeLocale::Zh, kind);
            let en = gate_dialog_strings(crate::native_i18n::NativeLocale::En, kind);
            assert!(!zh.title.is_empty() && !zh.description.is_empty());
            assert!(!en.title.is_empty() && !en.description.is_empty());
            assert_ne!(zh.title, en.title, "zh/en 标题不应相同: {:?}", kind);
        }
    }
}
