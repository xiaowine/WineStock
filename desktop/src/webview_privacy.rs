//! Desktop WebView 隐私策略。
//!
//! 普通表单自动填充由 Tauri 的 WebView builder 统一关闭；Windows 密码自动保存
//! 目前没有对应的 Tauri 高层选项，因此通过 `with_webview` 访问 WebView2 Settings。
//! 本模块不清理历史 Profile 数据，也不触碰应用自己的 localStorage/IndexedDB。

#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};

use tauri::WebviewWindow;

#[cfg(target_os = "windows")]
use windows::core::Interface;

/// 关闭 Windows WebView2 的密码自动保存；其它平台没有额外宿主层操作。
pub fn disable_password_autosave(window: &WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let error_slot = Arc::new(Mutex::new(None::<String>));
        let error_slot_for_callback = error_slot.clone();

        window
            .with_webview(move |webview| {
                let result = (|| -> Result<(), String> {
                    unsafe {
                    let core_webview = webview
                        .controller()
                        .CoreWebView2()
                        .map_err(|error| error.to_string())?;
                    let settings = core_webview
                        .Settings()
                        .map_err(|error| error.to_string())?;
                    let settings4 = settings
                        .cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings4>()
                        .map_err(|error| error.to_string())?;
                    settings4
                        .SetIsPasswordAutosaveEnabled(false)
                        .map_err(|error| error.to_string())
                    }
                })();

                if let Err(error) = result {
                    if let Ok(mut slot) = error_slot_for_callback.lock() {
                        *slot = Some(error);
                    }
                }
            })
            .map_err(|error| format!("无法访问 WebView2：{error}"))?;

        let callback_error = {
            let slot = error_slot
                .lock()
                .map_err(|_| "无法读取 WebView2 隐私设置结果".to_string())?;
            slot.clone()
        };
        if let Some(error) = callback_error {
            return Err(format!("无法关闭 WebView2 密码自动保存：{error}"));
        }
    }

    Ok(())
}
