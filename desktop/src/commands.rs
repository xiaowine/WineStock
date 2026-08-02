//! Tauri command 层：把 Shell Bridge v1 方法映射到 DesktopRuntimeManager。
//!
//! 本模块只做参数校验、状态查询、首屏就绪窗口协作与错误序列化，不包含业务逻辑。

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::{
    contract::{ApplyRuntimeConfigResult, EditableRuntimeConfig, RuntimeConfigValidationResult},
    runtime::DesktopRuntimeManager,
};

type CommandResult<T> = Result<T, String>;

static FRONTEND_READY: AtomicBool = AtomicBool::new(false);

/// 返回当前进程是否已收到前端首帧就绪信号；仅供 Desktop 窗口显示兜底使用。
pub fn is_frontend_ready() -> bool {
    FRONTEND_READY.load(Ordering::Acquire)
}

fn command_error(code: &str, message: &str) -> String {
    serde_json::json!({ "code": code, "message": message }).to_string()
}

#[tauri::command]
pub async fn shell_get_runtime_snapshot(
    manager: State<'_, Arc<DesktopRuntimeManager>>,
) -> CommandResult<crate::contract::RuntimeSnapshot> {
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
pub async fn shell_frontend_ready(app: AppHandle) -> CommandResult<()> {
    // 只有前端完成首帧渲染后才显示主窗口，避免 WebView 加载期间出现白屏或闪烁。
    FRONTEND_READY.store(true, Ordering::Release);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub async fn shell_open_external(app: AppHandle, url: String) -> CommandResult<()> {
    let parsed =
        Url::parse(&url).map_err(|_| command_error("invalid_bridge_payload", "外部链接无效"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(command_error(
            "invalid_bridge_payload",
            "外部链接必须使用不含凭据的 http 或 https 地址",
        ));
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|error| {
            command_error(
                "service_start_failed",
                &format!("无法打开外部链接：{error}"),
            )
        })?;
    Ok(())
}
