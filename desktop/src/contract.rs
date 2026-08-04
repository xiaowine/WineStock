//! Desktop Shell 与 frontend 之间版本化 Shell Bridge DTO。
//!
//! 本模块属于 `desktop` 壳的传输边界，镜像
//! `frontend/src/shell/contract.ts` 的字段语义，不启动 Axum、不读取业务数据。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 当前 Shell Bridge 协议版本，与 frontend `SHELL_BRIDGE_PROTOCOL_VERSION` 一致。
pub const SHELL_BRIDGE_PROTOCOL_VERSION: u32 = 1;

/// shared 支持的四类运行模式；使用 kebab-case 稳定字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    /// 只连接远端服务，不启动本地 Axum。
    ClientOnly,
    /// 本机自托管，启动本地 Axum 供本机 UI 使用。
    SelfHosted,
    /// 启动本地 Axum 作为可被其它设备访问的服务端。
    ServerMode,
    /// 连接远端服务；语义上保留给需要明确远端连接的客户端壳。
    ConnectToRemote,
}

/// Desktop 主窗口主题偏好；Windows Shell 将其映射到 Tauri 原生窗口主题。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowTheme {
    System,
    Light,
    Dark,
}

impl RuntimeMode {
    /// 仅连接远端服务、不启动本地 Axum 的模式。
    pub fn is_remote(self) -> bool {
        matches!(self, Self::ClientOnly | Self::ConnectToRemote)
    }

    /// 需要本地 Axum 的模式。
    #[allow(dead_code)]
    pub fn is_local(self) -> bool {
        !self.is_remote()
    }
}

/// 前端可编辑运行配置在 Shell Bridge 边界的稳定镜像。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditableRuntimeConfig {
    /// 运行模式稳定字符串。
    pub mode: String,
    /// 本地 Axum 监听地址；`0.0.0.0` 只能用于绑定，不能作为访问 URL。
    pub bind_host: String,
    /// 监听端口；先用有符号整数接收，以便稳定返回字段错误。
    pub port: i64,
    /// 远端客户端模式使用的 API 根地址。
    pub remote_base_url: String,
}

impl EditableRuntimeConfig {
    /// Shell 首次未初始化时的默认表单草稿，与 frontend `defaultRuntimeConfig` 一致。
    pub fn default_draft() -> Self {
        Self {
            mode: "self-hosted".to_owned(),
            bind_host: "127.0.0.1".to_owned(),
            port: 17890,
            remote_base_url: String::new(),
        }
    }
}

/// Shell Bridge 稳定错误码，与 `docs/shell-bridge.md` 一致。
pub const ERROR_CONFIG_UNAVAILABLE: &str = "config_unavailable";
pub const ERROR_CONFIG_INVALID: &str = "config_invalid";
pub const ERROR_STORAGE_UNAVAILABLE: &str = "storage_unavailable";
pub const ERROR_DATABASE_OPEN_FAILED: &str = "database_open_failed";
pub const ERROR_PORT_IN_USE: &str = "port_in_use";
pub const ERROR_SERVICE_START_FAILED: &str = "service_start_failed";
pub const ERROR_SERVICE_CRASHED: &str = "service_crashed";
pub const ERROR_UNSUPPORTED_RUNTIME_MODE: &str = "unsupported_runtime_mode";
pub const ERROR_FIREWALL_AUTHORIZATION_REQUIRED: &str = "firewall_authorization_required";
pub const ERROR_FIREWALL_POLICY_BLOCKED: &str = "firewall_policy_blocked";
pub const ERROR_FIREWALL_PROFILE_UNSUPPORTED: &str = "firewall_profile_unsupported";
pub const ERROR_FIREWALL_SERVICE_UNAVAILABLE: &str = "firewall_service_unavailable";
pub const ERROR_FIREWALL_RULE_UPDATE_FAILED: &str = "firewall_rule_update_failed";
pub const ERROR_FIREWALL_CLEANUP_PENDING: &str = "firewall_cleanup_pending";

/// Shell 可安全返回给前端的稳定运行错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellRuntimeError {
    /// 前端分支使用的稳定错误码。
    pub code: String,
    /// 面向用户的安全错误提示；不得包含敏感路径或凭据。
    pub message: String,
    /// 错误对应的运行配置字段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Windows 防火墙对当前 server-mode 端口的保护状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFirewallSnapshot {
    /// `ready`、`requires-elevation`、`blocked-by-policy`、`profile-unsupported`、`disabled` 或 `error`。
    pub status: String,
    /// 规则对应的当前服务端口。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// 规则设计使用的网络范围；当前 Windows 实现为 `local-subnet`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl ShellRuntimeError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }
}

/// 当前服务运行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeServiceSnapshot {
    /// 本地服务由 Shell 管理，远端服务只由前端执行 HTTP 检查。
    pub ownership: String,
    /// Shell 观察到的生命周期阶段。
    pub phase: String,
    /// 前端实际使用的 API 根地址。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,
    /// 本地服务真实监听地址，仅用于状态展示。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_address: Option<String>,
    /// self-hosted 本机静默会话换取凭据；仅 local+running 快照携带，不得写入日志。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_auth_exchange_token: Option<String>,
    /// server-mode 由 Shell 根据真实网卡地址发布的局域网访问地址。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_access_urls: Option<Vec<String>>,
    /// server-mode 当前平台防火墙保护状态。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firewall: Option<RuntimeFirewallSnapshot>,
    /// 最近一次配置或生命周期错误。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ShellRuntimeError>,
}

/// 平台能力集合；前端只能依据这些字段决定是否展示平台操作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub start_local_service: bool,
    pub stop_local_service: bool,
    pub restart_local_service: bool,
    pub native_back: bool,
    pub open_external: bool,
    pub server_mode: bool,
}

/// 当前生效配置、服务状态和平台能力的统一快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    /// Shell Bridge 协议版本。
    pub protocol_version: u32,
    /// 当前宿主平台。
    pub platform: String,
    /// 配置文件状态。
    pub config_status: String,
    /// 当前生效或待修复的可编辑配置。
    pub config: EditableRuntimeConfig,
    /// Shell 是否已有权威初始化配置；交互式平台通常由成功应用并持久化产生。
    pub initialized: bool,
    /// 当前服务运行状态。
    pub service: RuntimeServiceSnapshot,
    /// 平台能力。
    pub capabilities: RuntimeCapabilities,
}

/// 配置校验返回的字段错误集合。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigValidationResult {
    /// 全部字段是否通过校验。
    pub valid: bool,
    /// 按稳定字段名称聚合的错误。
    pub field_errors: BTreeMap<String, Vec<String>>,
    /// 校验通过后由 Shell 规范化的配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_config: Option<EditableRuntimeConfig>,
}

/// 保存并应用运行配置的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRuntimeConfigResult {
    /// 全部字段是否通过校验。
    pub valid: bool,
    /// 按稳定字段名称聚合的错误。
    pub field_errors: BTreeMap<String, Vec<String>>,
    /// Shell 是否成功激活并持久化配置。
    pub applied: bool,
    /// 成功或失败后 Shell 的权威快照。
    pub snapshot: RuntimeSnapshot,
    /// 非字段运行错误，例如端口占用或本地服务启动失败。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ShellRuntimeError>,
}

/// Desktop 能力：按当前快照的本地归属动态开放生命周期；当前仅 Windows 自动支持 server-mode。
pub fn desktop_capabilities(initialized: bool, ownership: &str) -> RuntimeCapabilities {
    let local_lifecycle_available = initialized && ownership == "local";
    RuntimeCapabilities {
        start_local_service: local_lifecycle_available,
        stop_local_service: local_lifecycle_available,
        restart_local_service: local_lifecycle_available,
        native_back: false,
        open_external: true,
        server_mode: cfg!(windows),
    }
}

/// 构建 stopped 服务快照。
pub fn stopped_service() -> RuntimeServiceSnapshot {
    RuntimeServiceSnapshot {
        ownership: "local".to_owned(),
        phase: "stopped".to_owned(),
        api_base_url: None,
        bound_address: None,
        local_auth_exchange_token: None,
        lan_access_urls: None,
        firewall: None,
        error: None,
    }
}
