//! Desktop 主窗口与 WebView 生命周期。
//!
//! 本模块拥有主 `WebviewWindow` 的创建、恢复、隐藏后的空闲回收和重新加载；不拥有
//! `DesktopRuntimeManager` 或 Axum 服务的启动、停止和业务请求。

use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::{
    commands,
    lifecycle::{AppLifecycleState, DebugStartupOverrides, WebviewState},
    preferences::DesktopPreferencesState,
    webview_privacy,
};

const MAIN_WINDOW_LABEL: &str = "main";
const MAIN_WINDOW_TITLE: &str = "WineStock";
const FRONTEND_LOAD_TIMEOUT: Duration = Duration::from_secs(8);

/// 由 Desktop Shell 保存的主窗口初始化配置；不包含业务数据或认证信息。
#[derive(Debug, Clone)]
pub struct MainWindowConfig {
    pub initialization_script: String,
}

impl MainWindowConfig {
    pub fn new(initialization_script: String) -> Self {
        Self {
            initialization_script,
        }
    }
}

/// 创建一代新的主 WebView，并启动该代次的前端握手超时保护。
pub fn create_main_window(app: &AppHandle, show_on_ready: bool) -> Result<WebviewWindow, String> {
    let lifecycle = app.state::<AppLifecycleState>();
    let generation = lifecycle.begin_webview_restore(show_on_ready);
    let base_initialization_script = app
        .state::<MainWindowConfig>()
        .initialization_script
        .clone();
    let initialization_script = format!(
        "{base_initialization_script}\nwindow.__WINESTOCK_WEBVIEW_GENERATION__ = {generation};"
    );
    commands::begin_frontend_load(generation);

    let window = match WebviewWindowBuilder::new(
        app,
        MAIN_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .initialization_script(initialization_script)
    .title(MAIN_WINDOW_TITLE)
    .inner_size(1280.0, 800.0)
    .min_inner_size(460.0, 600.0)
    .resizable(true)
    .center()
    .visible(false)
    .general_autofill_enabled(false)
    .build()
    {
        Ok(window) => window,
        Err(error) => {
            lifecycle.mark_webview_restore_failed(generation);
            return Err(format!("无法创建 WineStock 主窗口：{error}"));
        }
    };

    if let Err(error) = webview_privacy::disable_password_autosave(&window) {
        let _ = window.destroy();
        lifecycle.mark_webview_restore_failed(generation);
        return Err(format!("无法配置 WebView2 隐私设置：{error}"));
    }

    commands::spawn_frontend_load_timeout(app.clone(), generation, FRONTEND_LOAD_TIMEOUT);
    Ok(window)
}

/// 显示现有主窗口；恢复显示会使当前 generation 的回收任务失效。
pub fn show_main_window(window: &WebviewWindow) {
    let lifecycle = window.app_handle().state::<AppLifecycleState>();
    lifecycle.mark_webview_visible();
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// 确保主窗口存在并显示；窗口已经被回收时重新加载打包前端。
pub fn ensure_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        show_main_window(&window);
        return Ok(());
    }

    create_main_window(app, true).map(|_| ())
}

/// 托盘、单实例恢复和前端首帧显示共用的恢复入口。
pub fn show_main_window_by_label(app: &AppHandle) {
    if let Err(error) = ensure_main_window(app) {
        eprintln!("WineStock 主窗口恢复失败：{error}");
    }
}

/// 标记窗口隐藏并按当前 Desktop 偏好安排 WebView 空闲回收。
pub fn hide_main_window_and_schedule_reclaim(app: &AppHandle) {
    let lifecycle = app.state::<AppLifecycleState>();
    let generation = lifecycle.webview_generation();
    if lifecycle.webview_state() != WebviewState::Disposing {
        lifecycle.mark_webview_hidden(generation);
    }
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
    schedule_webview_reclaim(app);
}

/// 丢弃恢复失败的 WebView，使下一次托盘打开一定创建新页面。
pub fn discard_failed_webview(app: &AppHandle, generation: u64) {
    let lifecycle = app.state::<AppLifecycleState>();
    if !lifecycle.mark_webview_restore_failed(generation) {
        return;
    }
    let token = lifecycle.invalidate_webview_reclaim();
    if !lifecycle.begin_webview_reclaim(generation, token) {
        return;
    }

    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        let _ = lifecycle.complete_webview_reclaim(generation);
        return;
    };

    if let Err(error) = window.destroy() {
        eprintln!("WineStock 恢复失败的 WebView 销毁失败：{error}");
        let _ = lifecycle.mark_webview_hidden(generation);
        return;
    }
    let _ = lifecycle.complete_webview_reclaim(generation);
}

/// 按当前偏好安排回收。每次调用都会使之前的任务失效，避免重复销毁。
pub fn schedule_webview_reclaim(app: &AppHandle) {
    let lifecycle = app.state::<AppLifecycleState>();
    let token = lifecycle.invalidate_webview_reclaim();
    let generation = lifecycle.webview_generation();
    let preferences = app.state::<DesktopPreferencesState>().get();

    if !preferences.webview_reclaim_enabled || lifecycle.webview_state() != WebviewState::Hidden {
        return;
    }

    let delay = app
        .state::<DebugStartupOverrides>()
        .webview_reclaim_idle_seconds
        .map(Duration::from_secs)
        .unwrap_or_else(|| {
            Duration::from_secs(preferences.webview_reclaim_idle_minutes as u64 * 60)
        });
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;

        let lifecycle = handle.state::<AppLifecycleState>();
        if !lifecycle.begin_webview_reclaim(generation, token) {
            return;
        }

        let Some(window) = handle.get_webview_window(MAIN_WINDOW_LABEL) else {
            let _ = lifecycle.complete_webview_reclaim(generation);
            return;
        };

        match window.is_visible() {
            Ok(true) | Err(_) => {
                lifecycle.mark_webview_visible();
            }
            Ok(false) => match window.destroy() {
                Ok(()) => {
                    let _ = lifecycle.complete_webview_reclaim(generation);
                }
                Err(error) => {
                    eprintln!("WineStock WebView 空闲回收失败：{error}");
                    let _ = lifecycle.mark_webview_hidden(generation);
                }
            },
        }
    });
}
