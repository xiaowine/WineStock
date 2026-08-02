//! Desktop 主窗口的恢复操作。
//!
//! 托盘、单实例恢复和前端首帧显示共用这里的幂等窗口操作，不负责服务生命周期。

use tauri::{Manager, WebviewWindow};

pub fn show_main_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn show_main_window_by_label(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        show_main_window(&window);
    }
}
