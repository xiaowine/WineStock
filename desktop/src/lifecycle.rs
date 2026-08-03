//! Desktop 窗口/进程生命周期状态。
//!
//! 本模块区分普通关闭、托盘退出和应用退出清理，避免隐藏窗口时误停本地服务。

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

pub const AUTOSTART_LAUNCH_ARGUMENT: &str = "--winestock-autostart";
pub const FORCE_WEBVIEW_BLOCK_ARGUMENT: &str = "--winestock-force-webview-block";
pub const FORCE_SHELL_BRIDGE_BLOCK_ARGUMENT: &str = "--winestock-force-shell-bridge-block";
pub const FORCE_SHELL_BRIDGE_HANDSHAKE_BLOCK_ARGUMENT: &str =
    "--winestock-force-shell-bridge-handshake-block";
pub const WEBVIEW_RECLAIM_IDLE_SECONDS_ARGUMENT: &str = "--winestock-webview-reclaim-idle-seconds";
const WEBVIEW_RECLAIM_IDLE_SECONDS_ARGUMENT_PREFIX: &str =
    "--winestock-webview-reclaim-idle-seconds=";
const MAX_WEBVIEW_RECLAIM_IDLE_SECONDS: u64 = 86_400;

/// 启动测试覆盖；故障注入仅 Debug 构建启用，回收秒数覆盖不写入偏好文件。
#[derive(Debug, Clone, Copy, Default)]
pub struct DebugStartupOverrides {
    pub force_webview_block: bool,
    pub force_shell_bridge_block: bool,
    pub force_shell_bridge_handshake_block: bool,
    pub webview_reclaim_idle_seconds: Option<u64>,
}

pub fn debug_startup_overrides() -> DebugStartupOverrides {
    debug_startup_overrides_from(std::env::args())
}

fn debug_startup_overrides_from<I, S>(args: I) -> DebugStartupOverrides
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments: Vec<String> = args
        .into_iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    let webview_reclaim_idle_seconds = parse_webview_reclaim_idle_seconds(&arguments);

    #[cfg(not(debug_assertions))]
    {
        return DebugStartupOverrides {
            webview_reclaim_idle_seconds,
            ..Default::default()
        };
    }

    #[cfg(debug_assertions)]
    {
        let mut overrides = DebugStartupOverrides::default();
        overrides.webview_reclaim_idle_seconds = webview_reclaim_idle_seconds;
        for argument in arguments {
            match argument.as_str() {
                FORCE_WEBVIEW_BLOCK_ARGUMENT => overrides.force_webview_block = true,
                FORCE_SHELL_BRIDGE_BLOCK_ARGUMENT => overrides.force_shell_bridge_block = true,
                FORCE_SHELL_BRIDGE_HANDSHAKE_BLOCK_ARGUMENT => {
                    overrides.force_shell_bridge_handshake_block = true
                }
                _ => {}
            }
        }
        overrides
    }
}

fn parse_webview_reclaim_idle_seconds(arguments: &[String]) -> Option<u64> {
    for (index, argument) in arguments.iter().enumerate() {
        let value = argument
            .strip_prefix(WEBVIEW_RECLAIM_IDLE_SECONDS_ARGUMENT_PREFIX)
            .or_else(|| {
                (argument == WEBVIEW_RECLAIM_IDLE_SECONDS_ARGUMENT)
                    .then(|| arguments.get(index + 1).map(String::as_str))
                    .flatten()
            });
        let Some(value) = value else {
            continue;
        };
        let Ok(seconds) = value.parse::<u64>() else {
            continue;
        };
        if (1..=MAX_WEBVIEW_RECLAIM_IDLE_SECONDS).contains(&seconds) {
            return Some(seconds);
        }
    }
    None
}

/// 判断当前进程是否由 Tauri autostart 注册项拉起。
pub fn is_autostart_launch() -> bool {
    std::env::args().any(|argument| argument == AUTOSTART_LAUNCH_ARGUMENT)
}

/// Desktop 主 WebView 的进程内生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WebviewState {
    Alive = 0,
    Hidden = 1,
    Disposing = 2,
    Disposed = 3,
    Restoring = 4,
}

impl WebviewState {
    fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::Alive,
            1 => Self::Hidden,
            2 => Self::Disposing,
            4 => Self::Restoring,
            _ => Self::Disposed,
        }
    }
}

#[derive(Debug, Default)]
pub struct AppLifecycleState {
    close_allowed: AtomicBool,
    exit_started: AtomicBool,
    tray_available: AtomicBool,
    startup_silent: AtomicBool,
    webview_state: AtomicU8,
    webview_generation: AtomicU64,
    webview_reclaim_token: AtomicU64,
    show_webview_on_ready: AtomicBool,
    webview_exit_guard: AtomicBool,
}

impl AppLifecycleState {
    pub fn allow_close(&self) {
        self.close_allowed.store(true, Ordering::Release);
        self.webview_exit_guard.store(false, Ordering::Release);
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

    /// 为一次新的 WebView 加载分配代次，并取消旧回收任务。
    pub fn begin_webview_restore(&self, show_on_ready: bool) -> u64 {
        self.invalidate_webview_reclaim();
        self.webview_exit_guard.store(false, Ordering::Release);
        self.show_webview_on_ready
            .store(show_on_ready, Ordering::Release);
        self.webview_state
            .store(WebviewState::Restoring as u8, Ordering::Release);
        self.webview_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn webview_generation(&self) -> u64 {
        self.webview_generation.load(Ordering::Acquire)
    }

    pub fn webview_state(&self) -> WebviewState {
        WebviewState::from_raw(self.webview_state.load(Ordering::Acquire))
    }

    pub fn show_webview_on_ready(&self) -> bool {
        self.show_webview_on_ready.load(Ordering::Acquire)
    }

    pub fn mark_webview_ready(&self, generation: u64) -> bool {
        if self.webview_generation() != generation {
            return false;
        }
        self.webview_state
            .store(WebviewState::Alive as u8, Ordering::Release);
        true
    }

    pub fn mark_webview_hidden(&self, generation: u64) -> bool {
        if self.webview_generation() != generation {
            return false;
        }
        self.webview_exit_guard.store(false, Ordering::Release);
        self.webview_state
            .store(WebviewState::Hidden as u8, Ordering::Release);
        true
    }

    pub fn mark_webview_visible(&self) {
        self.invalidate_webview_reclaim();
        self.webview_exit_guard.store(false, Ordering::Release);
        self.webview_state
            .store(WebviewState::Alive as u8, Ordering::Release);
    }

    pub fn mark_webview_restore_failed(&self, generation: u64) -> bool {
        self.mark_webview_hidden(generation)
    }

    /// 使所有已经排队的回收任务失效，并返回新的 token。
    pub fn invalidate_webview_reclaim(&self) -> u64 {
        self.webview_reclaim_token.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn begin_webview_reclaim(&self, generation: u64, token: u64) -> bool {
        if self.webview_generation() != generation
            || self.webview_reclaim_token.load(Ordering::Acquire) != token
        {
            return false;
        }
        let changed = self
            .webview_state
            .compare_exchange(
                WebviewState::Hidden as u8,
                WebviewState::Disposing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok();
        if changed {
            self.webview_exit_guard.store(true, Ordering::Release);
        }
        changed
    }

    pub fn complete_webview_reclaim(&self, generation: u64) -> bool {
        if self.webview_generation() != generation {
            return false;
        }
        self.webview_state
            .store(WebviewState::Disposed as u8, Ordering::Release);
        true
    }

    pub fn webview_dispose_started(&self) -> bool {
        self.webview_exit_guard.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn parses_debug_startup_overrides() {
        let overrides = debug_startup_overrides_from([
            "winestock",
            FORCE_WEBVIEW_BLOCK_ARGUMENT,
            FORCE_SHELL_BRIDGE_BLOCK_ARGUMENT,
            FORCE_SHELL_BRIDGE_HANDSHAKE_BLOCK_ARGUMENT,
        ]);
        assert!(overrides.force_webview_block);
        assert!(overrides.force_shell_bridge_block);
        assert!(overrides.force_shell_bridge_handshake_block);
    }

    #[test]
    fn parses_webview_reclaim_idle_seconds_in_both_argument_forms() {
        let equals = debug_startup_overrides_from([
            "winestock",
            "--winestock-webview-reclaim-idle-seconds=10",
        ]);
        assert_eq!(equals.webview_reclaim_idle_seconds, Some(10));

        let separated = debug_startup_overrides_from([
            "winestock",
            WEBVIEW_RECLAIM_IDLE_SECONDS_ARGUMENT,
            "15",
        ]);
        assert_eq!(separated.webview_reclaim_idle_seconds, Some(15));

        let invalid = debug_startup_overrides_from([
            "winestock",
            "--winestock-webview-reclaim-idle-seconds=0",
        ]);
        assert_eq!(invalid.webview_reclaim_idle_seconds, None);
    }

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

    #[test]
    fn webview_generation_invalidates_old_reclaim_tasks() {
        let state = AppLifecycleState::default();
        let first = state.begin_webview_restore(true);
        state.mark_webview_ready(first);
        state.mark_webview_hidden(first);
        let token = state.invalidate_webview_reclaim();
        let second = state.begin_webview_restore(true);

        assert_ne!(first, second);
        assert!(!state.begin_webview_reclaim(first, token));
        assert_eq!(state.webview_state(), WebviewState::Restoring);
    }

    #[test]
    fn webview_reclaim_does_not_start_application_exit() {
        let state = AppLifecycleState::default();
        let generation = state.begin_webview_restore(true);
        state.mark_webview_ready(generation);
        state.mark_webview_hidden(generation);
        let token = state.invalidate_webview_reclaim();

        assert!(state.begin_webview_reclaim(generation, token));
        assert!(state.webview_dispose_started());
        assert!(state.complete_webview_reclaim(generation));
        assert_eq!(state.webview_state(), WebviewState::Disposed);
        assert!(!state.close_allowed());
        assert!(state.begin_exit());
    }
}
