//! Desktop 窗口/进程生命周期状态。
//!
//! 本模块区分普通关闭、托盘退出和应用退出清理，避免隐藏窗口时误停本地服务。

use std::sync::atomic::{AtomicBool, Ordering};

pub const AUTOSTART_LAUNCH_ARGUMENT: &str = "--winestock-autostart";

/// 判断当前进程是否由 Tauri autostart 注册项拉起。
pub fn is_autostart_launch() -> bool {
    std::env::args().any(|argument| argument == AUTOSTART_LAUNCH_ARGUMENT)
}

#[derive(Debug, Default)]
pub struct AppLifecycleState {
    close_allowed: AtomicBool,
    exit_started: AtomicBool,
    tray_available: AtomicBool,
    startup_silent: AtomicBool,
}

impl AppLifecycleState {
    pub fn allow_close(&self) {
        self.close_allowed.store(true, Ordering::Release);
    }

    pub fn close_allowed(&self) -> bool {
        self.close_allowed.load(Ordering::Acquire)
    }

    pub fn begin_exit(&self) -> bool {
        !self.exit_started.swap(true, Ordering::AcqRel)
    }

    pub fn set_tray_available(&self, available: bool) {
        self.tray_available.store(available, Ordering::Release);
    }

    pub fn tray_available(&self) -> bool {
        self.tray_available.load(Ordering::Acquire)
    }

    pub fn set_startup_silent(&self, silent: bool) {
        self.startup_silent.store(silent, Ordering::Release);
    }

    pub fn startup_silent(&self) -> bool {
        self.startup_silent.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_can_start_only_once() {
        let state = AppLifecycleState::default();
        assert!(state.begin_exit());
        assert!(!state.begin_exit());
    }

    #[test]
    fn close_permission_and_tray_state_are_independent() {
        let state = AppLifecycleState::default();
        assert!(!state.close_allowed());
        assert!(!state.tray_available());
        assert!(!state.startup_silent());
        state.allow_close();
        state.set_tray_available(true);
        assert!(state.close_allowed());
        assert!(state.tray_available());
    }

    #[test]
    fn startup_silent_state_is_independent() {
        let state = AppLifecycleState::default();
        state.set_startup_silent(true);
        assert!(state.startup_silent());
        state.set_startup_silent(false);
        assert!(!state.startup_silent());
    }
}
