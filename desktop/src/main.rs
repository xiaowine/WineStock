//! WineStock Desktop（Tauri v2）正式壳入口。
//!
//! 本壳拥有窗口、WebView、配置持久化与本地 Axum 生命周期；业务能力全部通过
//! HTTP 使用 `winestock-core`，不复制 core/server 实现。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use tauri::{Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use winestock_desktop::runtime::{
    emit_app_resumed, DesktopRuntimeManager, RUNTIME_STATE_CHANGED_EVENT,
};
use winestock_desktop::{
    lifecycle::{self, AppLifecycleState},
    preferences::{CloseBehavior, DesktopPreferencesState},
    tray, webview_compatibility, webview_debug, webview_privacy,
};

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

fn main() {
    if let Some(exit_code) = winestock_desktop::firewall::run_helper_if_requested() {
        std::process::exit(exit_code);
    }

    let debug_startup_overrides = lifecycle::debug_startup_overrides();

    webview_debug::configure();

    #[cfg(debug_assertions)]
    let prevent_default_plugin = tauri_plugin_prevent_default::debug();

    #[cfg(not(debug_assertions))]
    let prevent_default_plugin = tauri_plugin_prevent_default::init();

    tauri::Builder::default()
        .manage(AppLifecycleState::default())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![lifecycle::AUTOSTART_LAUNCH_ARGUMENT]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                winestock_desktop::window::show_main_window(&window);
            }
        }))
        .plugin(prevent_default_plugin)
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let webview = webview_compatibility::check();
            let webview_failure = if debug_startup_overrides.force_webview_block {
                Some(webview_compatibility::WebViewRuntimeFailure::ForcedBlock)
            } else {
                webview.failure
            };
            if let Some(failure) = webview_failure {
                eprintln!(
                    "WineStock Desktop 启动门卫失败：gate=webview2 reason={} version={:?} debug={}",
                    failure.code(),
                    webview.version,
                    cfg!(debug_assertions)
                );
                let (title, description) = match failure {
                    webview_compatibility::WebViewRuntimeFailure::Missing => {
                        (
                            "无法启动 WineStock",
                            "未检测到 WineStock 所需的 WebView2 Runtime。请重新安装 WineStock，安装器会补全所需组件。确认后 WineStock 将退出。",
                        )
                    }
                    webview_compatibility::WebViewRuntimeFailure::VersionTooOld => {
                        (
                            "WineStock 运行组件版本过低",
                            "当前 WebView2 Runtime 版本低于 WineStock 的最低要求（M111）。请重新安装 WineStock，安装器会补全满足要求的组件。确认后 WineStock 将退出。",
                        )
                    }
                    webview_compatibility::WebViewRuntimeFailure::VersionInvalid => {
                        (
                            "无法检查 WineStock 运行组件",
                            "无法正确读取 WebView2 Runtime 版本。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。",
                        )
                    }
                    webview_compatibility::WebViewRuntimeFailure::VersionCheckFailed => {
                        (
                            "无法检查 WineStock 运行组件",
                            "WineStock 无法确认 WebView2 Runtime 是否可用。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。",
                        )
                    }
                    webview_compatibility::WebViewRuntimeFailure::ForcedBlock => {
                        (
                            "WebView2 门卫测试",
                            "当前为 Debug 测试配置，已模拟 WebView2 版本不满足要求。确认后 WineStock 将退出。",
                        )
                    }
                };
                let description = append_diagnostic_code(description, failure.diagnostic_code());
                MessageDialog::new()
                    .set_level(MessageLevel::Error)
                    .set_title(title)
                    .set_description(description)
                    .set_buttons(MessageButtons::Ok)
                    .show();
                std::process::exit(1);
            }
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("无法解析应用数据目录：{error}"))?;
            app.manage(debug_startup_overrides);
            app.manage(DesktopPreferencesState::load(
                app_data_dir.join("desktop-preferences.json"),
            ));
            let desktop_preferences = app.state::<DesktopPreferencesState>().get();
            app.state::<AppLifecycleState>().set_startup_silent(
                lifecycle::is_autostart_launch() && desktop_preferences.autostart_silent,
            );
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
            let runtime_metadata_script =
                winestock_desktop::device_metadata::build_runtime_config_script(
                    &winestock_desktop::device_metadata::resolve_device_name(),
                    &app.package_info().version.to_string(),
                );

            let main_window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .initialization_script(runtime_metadata_script)
                    .title("WineStock")
                    .inner_size(1280.0, 800.0)
                    .min_inner_size(460.0, 600.0)
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
            if let Err(error) = tray::setup(app) {
                eprintln!("WineStock 系统托盘初始化失败：{error}");
            }
            let fallback_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 正常路径由 frontendReady 控制显示；未完成握手时使用原生提示并退出，
                // 避免用户看到空白或半初始化窗口。
                tokio::time::sleep(Duration::from_secs(8)).await;
                if !winestock_desktop::commands::is_frontend_ready()
                    && !winestock_desktop::commands::is_frontend_failure_reported()
                {
                    winestock_desktop::commands::show_frontend_load_timeout(&fallback_handle);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let app_handle = window.app_handle();
                    let lifecycle = app_handle.state::<AppLifecycleState>();
                    let preferences = app_handle.state::<DesktopPreferencesState>();
                    if preferences.get().close_behavior == CloseBehavior::MinimizeToTray
                        && lifecycle.tray_available()
                        && !lifecycle.close_allowed()
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                WindowEvent::Focused(true) => {
                    emit_app_resumed(window.app_handle());
                }
                _ => {}
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
            winestock_desktop::commands::shell_get_desktop_preferences,
            winestock_desktop::commands::shell_set_desktop_preferences,
            winestock_desktop::commands::shell_frontend_ready,
            winestock_desktop::commands::shell_frontend_failed,
        ])
        .build(tauri::generate_context!())
        .expect("WineStock Desktop 构建失败")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                // 第一次请求：阻止立即退出，等待本地 Axum 优雅停止后显式退出。
                // 显式 exit 会再次触发 ExitRequested，此时必须放行，否则形成退出循环。
                let handle = APP_HANDLE
                    .get()
                    .expect("setup 完成后 AppHandle 必须已记录")
                    .clone();
                if !handle.state::<AppLifecycleState>().begin_exit() {
                    return;
                }
                api.prevent_exit();
                let manager = handle.state::<Arc<DesktopRuntimeManager>>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    manager.shutdown_local_service(Duration::from_secs(5)).await;
                    handle.exit(0);
                });
            }
        });
}

fn append_diagnostic_code(description: &str, diagnostic_code: &str) -> String {
    format!("{description}\n错误代码：{diagnostic_code}")
}

#[cfg(test)]
mod tests {
    use super::append_diagnostic_code;

    #[test]
    fn appends_diagnostic_code_on_a_separate_line() {
        assert_eq!(
            append_diagnostic_code("加载失败。", "SHELL_BRIDGE_READY_FAILED"),
            "加载失败。\n错误代码：SHELL_BRIDGE_READY_FAILED"
        );
    }
}
