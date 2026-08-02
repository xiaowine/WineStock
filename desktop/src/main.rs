//! WineStock Desktop（Tauri v2）正式壳入口。
//!
//! 本壳拥有窗口、WebView、配置持久化与本地 Axum 生命周期；业务能力全部通过
//! HTTP 使用 `winestock-core`，不复制 core/server 实现。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use std::time::Duration;

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tauri::{
    Emitter, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use winestock_desktop::runtime::{
    emit_app_resumed, DesktopRuntimeManager, RUNTIME_STATE_CHANGED_EVENT,
};
use winestock_desktop::{webview_compatibility, webview_privacy};

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

/// 恢复并聚焦唯一的主窗口；第二个实例的参数和工作目录不会转交给首个实例。
fn show_main_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn main() {
    if let Some(exit_code) = winestock_desktop::firewall::run_helper_if_requested() {
        std::process::exit(exit_code);
    }

    #[cfg(not(debug_assertions))]
    unsafe {
        std::env::remove_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS");
    }

    #[cfg(debug_assertions)]
    let prevent_default_plugin = tauri_plugin_prevent_default::debug();

    #[cfg(not(debug_assertions))]
    let prevent_default_plugin = tauri_plugin_prevent_default::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                show_main_window(&window);
            }
        }))
        .plugin(prevent_default_plugin)
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let webview = webview_compatibility::check();
            if !webview.supported {
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title("WineStock 无法启动")
                    .set_description("WineStock 依赖损坏，请重新安装软件后重试。")
                    .set_buttons(MessageButtons::Ok)
                    .show();
                std::process::exit(1);
            }
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("无法解析应用数据目录：{error}"))?;
            let manager = Arc::new(DesktopRuntimeManager::new(
                Some(app.handle().clone()),
                app_data_dir,
            ));
            app.manage(manager.clone());
            // 已初始化配置必须在首个前端快照前完成恢复，否则前端会把短暂的
            // configured+stopped 误判为需要进入运行设置。首次未配置、远端配置
            // 或启动失败均会快速返回，并由最终快照驱动对应的前端页面。
            tauri::async_runtime::block_on(manager.initialize());
            let snapshot = tauri::async_runtime::block_on(manager.snapshot());

            let main_window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("WineStock")
                    .inner_size(1280.0, 800.0)
                    .min_inner_size(760.0, 560.0)
                    .resizable(true)
                    .center()
                    .visible(false)
                    .general_autofill_enabled(false)
                    .build()
                    .map_err(|error| format!("无法创建 WineStock 主窗口：{error}"))?;
            webview_privacy::disable_password_autosave(&main_window)
                .map_err(|error| format!("无法配置 WebView2 隐私设置：{error}"))?;
            let _ = app.emit(RUNTIME_STATE_CHANGED_EVENT, snapshot);
            DesktopRuntimeManager::spawn_monitor(manager);
            let _ = APP_HANDLE.set(app.handle().clone());
            let fallback_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 正常路径由 frontendReady 控制显示；异常页面或桥初始化失败时，
                // 受控超时仍显示前端设置/错误页，避免窗口永久隐藏。
                tokio::time::sleep(Duration::from_secs(8)).await;
                if !winestock_desktop::commands::is_frontend_ready() {
                    if let Some(window) = fallback_handle.get_webview_window("main") {
                        show_main_window(&window);
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(true) = event {
                if window.label() == "main" {
                    emit_app_resumed(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            winestock_desktop::commands::shell_get_runtime_snapshot,
            winestock_desktop::commands::shell_validate_runtime_config,
            winestock_desktop::commands::shell_apply_runtime_config,
            winestock_desktop::commands::shell_start_local_service,
            winestock_desktop::commands::shell_stop_local_service,
            winestock_desktop::commands::shell_restart_local_service,
            winestock_desktop::commands::shell_repair_firewall,
            winestock_desktop::commands::shell_frontend_ready,
            winestock_desktop::commands::shell_open_external,
        ])
        .build(tauri::generate_context!())
        .expect("WineStock Desktop 构建失败")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                // 第一次请求：阻止立即退出，等待本地 Axum 优雅停止后显式退出。
                // 显式 exit 会再次触发 ExitRequested，此时必须放行，否则形成退出循环。
                if EXIT_REQUESTED.swap(true, Ordering::SeqCst) {
                    return;
                }
                api.prevent_exit();
                let handle = APP_HANDLE
                    .get()
                    .expect("setup 完成后 AppHandle 必须已记录")
                    .clone();
                let manager = handle.state::<Arc<DesktopRuntimeManager>>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    manager.shutdown_local_service(Duration::from_secs(5)).await;
                    handle.exit(0);
                });
            }
        });
}
