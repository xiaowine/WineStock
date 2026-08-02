//! Desktop 运行快照构造、core 启动结果映射和本机访问地址派生。

use super::config::parse_mode;
use crate::contract::{
    self, desktop_capabilities, stopped_service, EditableRuntimeConfig, RuntimeMode,
    RuntimeServiceSnapshot, RuntimeSnapshot, ShellRuntimeError, ERROR_CONFIG_INVALID,
    ERROR_DATABASE_OPEN_FAILED, ERROR_FIREWALL_AUTHORIZATION_REQUIRED,
    ERROR_FIREWALL_POLICY_BLOCKED, ERROR_FIREWALL_PROFILE_UNSUPPORTED,
    ERROR_FIREWALL_RULE_UPDATE_FAILED, ERROR_FIREWALL_SERVICE_UNAVAILABLE, ERROR_PORT_IN_USE,
    ERROR_SERVICE_CRASHED, ERROR_SERVICE_START_FAILED,
};
use crate::lan_access::discover_lan_access_urls;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use url::Url;
use winestock_core::{LocalServiceRuntimeError, RunningLocalService, ServerStartError};

/// 本地服务启动成功后的只读信息；只包含可安全返回给前端的字段。
#[derive(Debug, Clone)]
pub(crate) struct LocalServiceDetails {
    pub(crate) bound_address: String,
    pub(crate) api_base_url: String,
    pub(crate) local_auth_exchange_token: Option<String>,
    pub(crate) lan_access_urls: Vec<String>,
    pub(crate) firewall: Option<crate::contract::RuntimeFirewallSnapshot>,
    pub(crate) firewall_error: Option<ShellRuntimeError>,
}

pub(crate) fn unconfigured_snapshot(config: EditableRuntimeConfig) -> RuntimeSnapshot {
    RuntimeSnapshot {
        protocol_version: contract::SHELL_BRIDGE_PROTOCOL_VERSION,
        platform: "desktop".to_owned(),
        config_status: "unconfigured".to_owned(),
        config,
        initialized: false,
        service: stopped_service(),
        capabilities: desktop_capabilities(false, "local"),
    }
}

pub(crate) fn invalid_snapshot(config: EditableRuntimeConfig) -> RuntimeSnapshot {
    let mut snapshot = unconfigured_snapshot(config);
    snapshot.config_status = "invalid".to_owned();
    snapshot.service.error = Some(ShellRuntimeError::new(
        ERROR_CONFIG_INVALID,
        "运行配置文件损坏或校验失败",
    ));
    snapshot
}

pub(crate) fn configured_snapshot(
    config: EditableRuntimeConfig,
    service: RuntimeServiceSnapshot,
) -> RuntimeSnapshot {
    let ownership = service.ownership.clone();
    RuntimeSnapshot {
        protocol_version: contract::SHELL_BRIDGE_PROTOCOL_VERSION,
        platform: "desktop".to_owned(),
        config_status: "configured".to_owned(),
        config,
        initialized: true,
        service,
        capabilities: desktop_capabilities(true, &ownership),
    }
}

pub(crate) fn remote_snapshot(
    config: EditableRuntimeConfig,
    _initialized: bool,
) -> RuntimeSnapshot {
    let mut snapshot = configured_snapshot(config.clone(), stopped_service());
    snapshot.service = RuntimeServiceSnapshot {
        ownership: "remote".to_owned(),
        phase: "stopped".to_owned(),
        api_base_url: Some(config.remote_base_url.clone()),
        bound_address: None,
        local_auth_exchange_token: None,
        lan_access_urls: None,
        firewall: None,
        error: None,
    };
    snapshot.capabilities = desktop_capabilities(true, &snapshot.service.ownership);
    snapshot
}

pub(crate) fn local_running_snapshot(
    config: EditableRuntimeConfig,
    details: &LocalServiceDetails,
    _initialized: bool,
) -> RuntimeSnapshot {
    let mut snapshot = configured_snapshot(config.clone(), stopped_service());
    snapshot.service = RuntimeServiceSnapshot {
        ownership: "local".to_owned(),
        phase: "running".to_owned(),
        api_base_url: Some(details.api_base_url.clone()),
        bound_address: Some(details.bound_address.clone()),
        local_auth_exchange_token: details.local_auth_exchange_token.clone(),
        lan_access_urls: if parse_mode(&config.mode) == Some(RuntimeMode::ServerMode) {
            Some(details.lan_access_urls.clone())
        } else {
            None
        },
        firewall: details.firewall.clone(),
        error: details.firewall_error.clone(),
    };
    snapshot
}

pub(crate) fn failed_snapshot(
    config: EditableRuntimeConfig,
    error: ShellRuntimeError,
    _initialized: bool,
) -> RuntimeSnapshot {
    let mut snapshot = configured_snapshot(config.clone(), stopped_service());
    let firewall = firewall_snapshot_for_error(&config, &error);
    snapshot.service = RuntimeServiceSnapshot {
        ownership: "local".to_owned(),
        phase: "failed".to_owned(),
        api_base_url: None,
        bound_address: None,
        local_auth_exchange_token: None,
        lan_access_urls: None,
        firewall,
        error: Some(error),
    };
    snapshot
}

pub(crate) fn firewall_snapshot_for_error(
    config: &EditableRuntimeConfig,
    error: &ShellRuntimeError,
) -> Option<crate::contract::RuntimeFirewallSnapshot> {
    if config.mode != "server-mode" {
        return None;
    }
    let status = match error.code.as_str() {
        ERROR_FIREWALL_AUTHORIZATION_REQUIRED => "requires-elevation",
        ERROR_FIREWALL_POLICY_BLOCKED => "blocked-by-policy",
        ERROR_FIREWALL_PROFILE_UNSUPPORTED => "profile-unsupported",
        ERROR_FIREWALL_SERVICE_UNAVAILABLE => "disabled",
        ERROR_FIREWALL_RULE_UPDATE_FAILED => "error",
        _ => return None,
    };
    Some(crate::contract::RuntimeFirewallSnapshot {
        status: status.to_owned(),
        port: u16::try_from(config.port).ok(),
        scope: Some("local-subnet".to_owned()),
    })
}

pub(crate) fn apply_cleanup_status(
    snapshot: &mut RuntimeSnapshot,
    cleanup: Option<crate::contract::RuntimeFirewallSnapshot>,
) {
    if let Some(firewall) = cleanup {
        snapshot.service.firewall = Some(firewall);
        snapshot.service.error = Some(ShellRuntimeError::new(
            crate::contract::ERROR_FIREWALL_CLEANUP_PENDING,
            "旧的 Windows 防火墙规则清理未完成",
        ));
    }
}

pub(crate) fn effective_config(
    config: &EditableRuntimeConfig,
    details: &LocalServiceDetails,
) -> EditableRuntimeConfig {
    if parse_mode(&config.mode) != Some(RuntimeMode::SelfHosted) {
        return config.clone();
    }
    let actual_port = details
        .api_base_url
        .parse::<Url>()
        .ok()
        .and_then(|url| url.port());
    match actual_port {
        Some(port) if i64::from(port) != config.port => EditableRuntimeConfig {
            port: i64::from(port),
            ..config.clone()
        },
        _ => config.clone(),
    }
}

pub(crate) fn map_start_result(
    result: Result<RunningLocalService, LocalServiceRuntimeError>,
    server_mode: bool,
) -> Result<(RunningLocalService, LocalServiceDetails), ShellRuntimeError> {
    match result {
        Ok(service) => {
            let info = service.info().clone();
            Ok((
                service,
                LocalServiceDetails {
                    bound_address: info.bound_addr.to_string(),
                    api_base_url: local_api_base_url(info.bound_addr),
                    local_auth_exchange_token: info
                        .local_session_token
                        .as_ref()
                        .map(|token| token.expose().to_owned()),
                    lan_access_urls: if server_mode {
                        discover_lan_access_urls(info.bound_addr)
                    } else {
                        Vec::new()
                    },
                    firewall: None,
                    firewall_error: None,
                },
            ))
        }
        Err(error) => Err(map_service_error(&error)),
    }
}

/// 生成 Desktop WebView 使用的本机 API 地址。
///
/// wildcard 监听仍优先走 loopback；具体监听地址则必须使用实际绑定的 IP，否则 WebView
/// 无法访问只绑定在某个网卡上的服务。
pub(crate) fn local_api_base_url(bound_addr: SocketAddr) -> String {
    let ip = match bound_addr.ip() {
        IpAddr::V4(address) if address.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(address) if address.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        address => address,
    };
    format!("http://{}", SocketAddr::new(ip, bound_addr.port()))
}

pub(crate) fn map_service_error(error: &LocalServiceRuntimeError) -> ShellRuntimeError {
    match error {
        LocalServiceRuntimeError::Bootstrap(_) => {
            ShellRuntimeError::new(ERROR_DATABASE_OPEN_FAILED, "本地数据库初始化失败")
        }
        LocalServiceRuntimeError::LocalServiceNotInitialized => {
            ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务初始化失败")
        }
        LocalServiceRuntimeError::Server(ServerStartError::Bind { source, .. })
            if source.kind() == std::io::ErrorKind::AddrInUse =>
        {
            ShellRuntimeError::new(ERROR_PORT_IN_USE, "本地端口已被占用")
        }
        LocalServiceRuntimeError::Server(_) => {
            ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务启动失败")
        }
        LocalServiceRuntimeError::Task(_) => {
            ShellRuntimeError::new(ERROR_SERVICE_CRASHED, "本地服务意外退出")
        }
    }
}
