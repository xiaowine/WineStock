//! Android DTO 到 shared AppConfig 的权威映射与平台策略。
//!
//! shared 提供通用字段校验；本模块额外限制 Android 当前只支持 loopback self-hosted，
//! 并拒绝尚未实现 Foreground Service 的 server-mode。

use std::{collections::BTreeMap, net::IpAddr, path::Path};

use url::{Host, Url};
use winestock_shared::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};

use crate::{
    contract::{
        EditableRuntimeConfig, NativeValidationResult, RuntimeConfigRequest,
        NATIVE_PROTOCOL_VERSION,
    },
    error::NativeError,
};

/// 校验完成后供 engine 启动 core 使用的配置。
pub struct PreparedRuntimeConfig {
    /// 已通过 shared 与 Android 平台策略的完整配置。
    pub app_config: AppConfig,

    /// 返回 Kotlin/前端使用的规范化可编辑配置。
    pub normalized_config: EditableRuntimeConfig,
}

/// 校验请求并返回 Shell Bridge 字段错误；内部存储错误直接返回稳定 native error。
pub fn validate_runtime_request(
    request: &RuntimeConfigRequest,
) -> Result<NativeValidationResult, NativeError> {
    let prepared = prepare_runtime_config(request)?;
    Ok(match prepared {
        PrepareResult::Valid(prepared) => NativeValidationResult {
            valid: true,
            field_errors: BTreeMap::new(),
            normalized_config: Some(prepared.normalized_config),
        },
        PrepareResult::Invalid(field_errors) => NativeValidationResult {
            valid: false,
            field_errors,
            normalized_config: None,
        },
    })
}

/// 校验并取得可启动的 AppConfig；字段无效时返回 `config_invalid`。
pub fn require_runtime_config(
    request: &RuntimeConfigRequest,
) -> Result<PreparedRuntimeConfig, NativeError> {
    match prepare_runtime_config(request)? {
        PrepareResult::Valid(prepared) => Ok(prepared),
        PrepareResult::Invalid(field_errors) => Err(NativeError::config_invalid(
            field_errors.keys().next().map(String::as_str),
        )),
    }
}

enum PrepareResult {
    Valid(PreparedRuntimeConfig),
    Invalid(BTreeMap<String, Vec<String>>),
}

fn prepare_runtime_config(request: &RuntimeConfigRequest) -> Result<PrepareResult, NativeError> {
    if request.native_protocol_version != NATIVE_PROTOCOL_VERSION {
        return Err(NativeError::version_mismatch());
    }
    validate_storage_paths(request)?;

    let mut field_errors = BTreeMap::<String, Vec<String>>::new();
    let mode = parse_mode(&request.config.mode, &mut field_errors);
    let port = validate_port(request.config.port, mode, &mut field_errors);
    let bind_host = request.config.bind_host.trim().to_owned();
    let mut normalized_remote = request.config.remote_base_url.trim().to_owned();

    match mode {
        Some(RuntimeMode::SelfHosted) => validate_android_loopback(&bind_host, &mut field_errors),
        Some(RuntimeMode::ServerMode) => push_error(
            &mut field_errors,
            "mode",
            "当前 Android 版本尚未支持 server-mode",
        ),
        Some(RuntimeMode::ClientOnly | RuntimeMode::ConnectToRemote) => {
            match normalize_remote_url(&normalized_remote) {
                Ok(value) => normalized_remote = value,
                Err(message) => push_error(&mut field_errors, "remoteBaseUrl", message),
            }
        }
        None => {}
    }

    if !field_errors.is_empty() {
        return Ok(PrepareResult::Invalid(field_errors));
    }

    let mode = mode.expect("无字段错误时 mode 必须已解析");
    let port = port.expect("无字段错误时 port 必须已通过范围校验");
    let normalized_config = EditableRuntimeConfig {
        mode: request.config.mode.trim().to_owned(),
        bind_host: bind_host.clone(),
        port: i64::from(port),
        remote_base_url: normalized_remote.clone(),
    };
    let app_config = AppConfig {
        server: ServerConfig {
            mode,
            bind_host,
            port,
            auto_start_server: true,
            remote_base_url: normalized_remote,
        },
        storage: StorageConfig {
            database_path: request.storage.database_path.trim().to_owned(),
            files_dir: request.storage.files_dir.trim().to_owned(),
            auto_migrate: true,
        },
    };

    let shared_issues = app_config.validation_issues();
    for issue in shared_issues {
        match issue.path.as_str() {
            "server.bind_host" => push_error(
                &mut field_errors,
                "bindHost",
                "监听地址必须是有效的 IP 地址",
            ),
            "server.port" => push_error(
                &mut field_errors,
                "port",
                "端口必须是 1 到 65535 之间的整数",
            ),
            "server.remote_base_url" => push_error(
                &mut field_errors,
                "remoteBaseUrl",
                "远端服务地址必须使用 http 或 https",
            ),
            "storage.database_path" | "storage.files_dir" => {
                return Err(NativeError::new(
                    "storage_unavailable",
                    "Android 本地存储路径无效",
                ));
            }
            _ => return Err(NativeError::new("config_invalid", "运行配置校验失败")),
        }
    }

    if field_errors.is_empty() {
        Ok(PrepareResult::Valid(PreparedRuntimeConfig {
            app_config,
            normalized_config,
        }))
    } else {
        Ok(PrepareResult::Invalid(field_errors))
    }
}

fn validate_storage_paths(request: &RuntimeConfigRequest) -> Result<(), NativeError> {
    for value in [
        request.storage.database_path.trim(),
        request.storage.files_dir.trim(),
    ] {
        if value.is_empty() || !Path::new(value).is_absolute() {
            return Err(NativeError::new(
                "storage_unavailable",
                "Android 本地存储路径不可用",
            ));
        }
    }
    Ok(())
}

fn parse_mode(
    value: &str,
    field_errors: &mut BTreeMap<String, Vec<String>>,
) -> Option<RuntimeMode> {
    match value.trim() {
        "self-hosted" => Some(RuntimeMode::SelfHosted),
        "client-only" => Some(RuntimeMode::ClientOnly),
        "connect-to-remote" => Some(RuntimeMode::ConnectToRemote),
        "server-mode" => Some(RuntimeMode::ServerMode),
        _ => {
            push_error(field_errors, "mode", "请选择有效的运行方式");
            None
        }
    }
}

fn validate_port(
    value: i64,
    mode: Option<RuntimeMode>,
    field_errors: &mut BTreeMap<String, Vec<String>>,
) -> Option<u16> {
    match u16::try_from(value) {
        Ok(port) if port > 0 || mode == Some(RuntimeMode::SelfHosted) => Some(port),
        _ => {
            push_error(field_errors, "port", "端口必须是 1 到 65535 之间的整数");
            None
        }
    }
}

fn validate_android_loopback(value: &str, field_errors: &mut BTreeMap<String, Vec<String>>) {
    match value.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) if ip.is_loopback() && ip.octets() == [127, 0, 0, 1] => {}
        _ => push_error(
            field_errors,
            "bindHost",
            "当前 Android self-hosted 仅支持 127.0.0.1",
        ),
    }
}

fn normalize_remote_url(value: &str) -> Result<String, &'static str> {
    if value.trim().is_empty() {
        return Err("请输入远端服务 API 地址");
    }
    let parsed = Url::parse(value.trim()).map_err(|_| "远端服务地址无效")?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("远端服务地址必须使用 http 或 https");
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("远端服务地址不能包含凭据、查询参数或 hash");
    }
    match parsed.host() {
        None => return Err("远端服务地址必须包含主机"),
        Some(Host::Ipv4(ip)) if ip.is_unspecified() => {
            return Err("全接口监听地址不能作为前端访问地址")
        }
        Some(Host::Ipv6(ip)) if ip.is_unspecified() => {
            return Err("全接口监听地址不能作为前端访问地址")
        }
        _ => {}
    }

    Ok(parsed.to_string().trim_end_matches('/').to_owned())
}

fn push_error(field_errors: &mut BTreeMap<String, Vec<String>>, field: &str, message: &str) {
    let messages = field_errors.entry(field.to_owned()).or_default();
    if !messages.iter().any(|existing| existing == message) {
        messages.push(message.to_owned());
    }
}
