//! Desktop 系统托盘装配与交互。
//!
//! 托盘只负责恢复主窗口和请求应用退出，不读取运行配置、不代理业务 API。

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager,
};

use crate::{lifecycle::AppLifecycleState, window::show_main_window_by_label};

const OPEN_MENU_ID: &str = "open";
const QUIT_MENU_ID: &str = "quit";

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let Some(icon) = app.default_window_icon().cloned() else {
        app.state::<AppLifecycleState>().set_tray_available(false);
        return Ok(());
    };

    let open = MenuItemBuilder::with_id(OPEN_MENU_ID, "打开 WineStock").build(app)?;
    let quit = MenuItemBuilder::with_id(QUIT_MENU_ID, "退出 WineStock").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&open, &quit]).build()?;

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("WineStock")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_MENU_ID => show_main_window_by_label(app),
            QUIT_MENU_ID => request_exit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window_by_label(tray.app_handle());
            }
        })
        .build(app)?;

    app.state::<AppLifecycleState>().set_tray_available(true);
    Ok(())
}

/// 请求现有的 ExitRequested 清理流程；不直接停止 runtime manager。
fn request_exit(app: &AppHandle) {
    app.state::<AppLifecycleState>().allow_close();
    app.exit(0);
}
