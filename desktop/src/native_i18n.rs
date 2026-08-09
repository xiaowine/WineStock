//! Desktop 原生 UI 文案本地化（方案 A：sys-locale 探测系统语言 + 静态 zh/en 表）。
//!
//! 只覆盖 WebView 前端不可达的原生面：系统托盘菜单与启动门禁 MessageDialog。
//! 语言在启动时按系统 UI 语言解析一次（托盘菜单构建后不可热切，重启生效）；
//! 诊断码（`WEBVIEW2_MISSING`、`FRONTEND_LOAD_TIMEOUT` 等）不随语言变化。
//! 前端可访问的业务文案不属于本模块（由共享前端 i18n 承担）。

use crate::webview_compatibility::WebViewRuntimeFailure;

/// 启动时按系统 UI 语言解析的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLocale {
    /// 简体中文。
    Zh,
    /// 英文兜底。
    En,
}

/// 读取系统 UI 语言；无法解析或非中文时回退英文。
pub fn system_locale() -> NativeLocale {
    match sys_locale::get_locale() {
        Some(locale) if locale.to_ascii_lowercase().starts_with("zh") => NativeLocale::Zh,
        _ => NativeLocale::En,
    }
}

/// 系统托盘菜单文案。
pub struct TrayStrings {
    /// 打开主窗口菜单项。
    pub open: &'static str,
    /// 退出应用菜单项。
    pub quit: &'static str,
}

/// 返回当前语言下的托盘菜单文案。
pub fn tray_strings(locale: NativeLocale) -> TrayStrings {
    match locale {
        NativeLocale::Zh => TrayStrings {
            open: "打开 WineStock",
            quit: "退出 WineStock",
        },
        NativeLocale::En => TrayStrings {
            open: "Open WineStock",
            quit: "Quit WineStock",
        },
    }
}

/// 启动门禁失败弹窗文案。
pub struct GateDialogStrings {
    /// 弹窗标题。
    pub title: &'static str,
    /// 弹窗说明；调用方负责追加诊断码。
    pub description: &'static str,
}

/// 门禁失败种类；由原生失败分类映射，文案随语言变化、诊断码不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFailureKind {
    /// WebView2 Runtime 缺失。
    WebView2Missing,
    /// WebView2 Runtime 版本过低。
    WebView2VersionTooOld,
    /// WebView2 Runtime 版本无法读取。
    WebView2VersionInvalid,
    /// WebView2 Runtime 可用性无法确认。
    WebView2VersionCheckFailed,
    /// Debug 强制阻断测试。
    WebView2ForcedBlock,
    /// 前端页面加载超时。
    FrontendLoadTimeout,
    /// Shell Bridge 快照无效或协议版本不匹配。
    ShellBridgeMismatch,
    /// Shell Bridge 缺少必要方法或扩展能力。
    ShellBridgeMissingCapability,
    /// Shell Bridge 握手（订阅/就绪上报）失败。
    ShellBridgeHandshakeFailed,
    /// Shell Bridge 不可用或无其它分类。
    ShellBridgeUnavailable,
}

impl From<WebViewRuntimeFailure> for GateFailureKind {
    fn from(failure: WebViewRuntimeFailure) -> Self {
        match failure {
            WebViewRuntimeFailure::Missing => Self::WebView2Missing,
            WebViewRuntimeFailure::VersionTooOld => Self::WebView2VersionTooOld,
            WebViewRuntimeFailure::VersionInvalid => Self::WebView2VersionInvalid,
            WebViewRuntimeFailure::VersionCheckFailed => Self::WebView2VersionCheckFailed,
            WebViewRuntimeFailure::ForcedBlock => Self::WebView2ForcedBlock,
        }
    }
}

/// 返回当前语言下的门禁弹窗文案。
pub fn gate_dialog_strings(locale: NativeLocale, kind: GateFailureKind) -> GateDialogStrings {
    match (locale, kind) {
        (NativeLocale::Zh, GateFailureKind::WebView2Missing) => GateDialogStrings {
            title: "无法启动 WineStock",
            description: "未检测到 WineStock 所需的 WebView2 Runtime。请重新安装 WineStock，安装器会补全所需组件。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::WebView2VersionTooOld) => GateDialogStrings {
            title: "WineStock 运行组件版本过低",
            description: "当前 WebView2 Runtime 版本低于 WineStock 的最低要求（M111）。请重新安装 WineStock，安装器会补全满足要求的组件。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::WebView2VersionInvalid) => GateDialogStrings {
            title: "无法检查 WineStock 运行组件",
            description: "无法正确读取 WebView2 Runtime 版本。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::WebView2VersionCheckFailed) => GateDialogStrings {
            title: "无法检查 WineStock 运行组件",
            description: "WineStock 无法确认 WebView2 Runtime 是否可用。请重新安装 WineStock，安装器会重新配置所需组件。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::WebView2ForcedBlock) => GateDialogStrings {
            title: "WebView2 门卫测试",
            description: "当前为 Debug 测试配置，已模拟 WebView2 版本不满足要求。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::FrontendLoadTimeout) => GateDialogStrings {
            title: "WineStock 页面加载超时",
            description: "页面未能在规定时间内完成加载。请重新启动 WineStock；问题仍存在时请重新安装软件。确认后 WineStock 将退出。",
        },
        (NativeLocale::En, GateFailureKind::WebView2Missing) => GateDialogStrings {
            title: "WineStock Cannot Start",
            description: "The WebView2 Runtime required by WineStock was not detected. Reinstall WineStock and the installer will add the required component. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::WebView2VersionTooOld) => GateDialogStrings {
            title: "WineStock Runtime Component Is Too Old",
            description: "The installed WebView2 Runtime is below WineStock's minimum requirement (M111). Reinstall WineStock and the installer will install a compatible component. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::WebView2VersionInvalid) => GateDialogStrings {
            title: "Cannot Check WineStock Runtime Component",
            description: "WineStock could not read the WebView2 Runtime version. Reinstall WineStock and the installer will reconfigure the required component. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::WebView2VersionCheckFailed) => GateDialogStrings {
            title: "Cannot Check WineStock Runtime Component",
            description: "WineStock could not confirm whether the WebView2 Runtime is available. Reinstall WineStock and the installer will reconfigure the required component. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::WebView2ForcedBlock) => GateDialogStrings {
            title: "WebView2 Gate Test",
            description: "This is a Debug test configuration simulating an unsupported WebView2 version. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::FrontendLoadTimeout) => GateDialogStrings {
            title: "WineStock Page Load Timed Out",
            description: "The page did not finish loading in time. Restart WineStock; if the problem persists, reinstall the software. WineStock will exit after you confirm.",
        },
        (NativeLocale::Zh, GateFailureKind::ShellBridgeMismatch) => GateDialogStrings {
            title: "WineStock 无法加载",
            description: "当前界面与桌面运行组件版本不匹配。请重新安装同一版本的 WineStock。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::ShellBridgeMissingCapability) => GateDialogStrings {
            title: "WineStock 无法加载",
            description: "桌面运行组件缺少必要能力。请重新安装 WineStock。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::ShellBridgeHandshakeFailed) => GateDialogStrings {
            title: "WineStock 页面加载失败",
            description: "WineStock 页面无法完成启动握手。请重新启动软件；问题仍存在时请重新安装软件。确认后 WineStock 将退出。",
        },
        (NativeLocale::Zh, GateFailureKind::ShellBridgeUnavailable) => GateDialogStrings {
            title: "WineStock 无法连接桌面组件",
            description: "桌面运行组件没有正常响应。请重新启动 WineStock；问题仍存在时请修复或重新安装软件。确认后 WineStock 将退出。",
        },
        (NativeLocale::En, GateFailureKind::ShellBridgeMismatch) => GateDialogStrings {
            title: "WineStock Cannot Load",
            description: "The interface and the desktop runtime component versions do not match. Reinstall the same version of WineStock. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::ShellBridgeMissingCapability) => GateDialogStrings {
            title: "WineStock Cannot Load",
            description: "The desktop runtime component is missing a required capability. Reinstall WineStock. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::ShellBridgeHandshakeFailed) => GateDialogStrings {
            title: "WineStock Page Failed to Load",
            description: "The WineStock page could not complete its startup handshake. Restart the app; if the problem persists, reinstall it. WineStock will exit after you confirm.",
        },
        (NativeLocale::En, GateFailureKind::ShellBridgeUnavailable) => GateDialogStrings {
            title: "WineStock Cannot Reach the Desktop Component",
            description: "The desktop runtime component did not respond correctly. Restart WineStock; if the problem persists, repair or reinstall it. WineStock will exit after you confirm.",
        },
    }
}

/// 追加面向用户的诊断码标签；标签文本随语言变化，诊断码本身不翻译。
pub fn append_diagnostic_code(locale: NativeLocale, description: &str, diagnostic_code: &str) -> String {
    let label = match locale {
        NativeLocale::Zh => "错误代码",
        NativeLocale::En => "Error code",
    };
    format!("{description}\n{label}：{diagnostic_code}")
}
