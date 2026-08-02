//! Desktop 运行配置的校验、规范化、持久化和平台存储路径。

use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    net::IpAddr,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use url::Url;
use winestock_shared::{AppConfig, ServerConfig, StorageConfig};

use crate::contract::{
    EditableRuntimeConfig, RuntimeConfigValidationResult, RuntimeMode, ShellRuntimeError,
    ERROR_CONFIG_UNAVAILABLE, ERROR_STORAGE_UNAVAILABLE,
};

const DATABASE_FILE_NAME: &str = "winestock.sqlite";
const FILES_DIR_NAME: &str = "files";
const FIREWALL_STATE_FILE_NAME: &str = "firewall-state.json";

/// 配置文件的稳定 JSON 结构；只持久化前端可编辑字段，存储路径由平台派生。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PersistedRuntimeConfig {
    mode: String,
    bind_host: String,
    port: i64,
    remote_base_url: String,
}

impl From<&EditableRuntimeConfig> for PersistedRuntimeConfig {
    fn from(value: &EditableRuntimeConfig) -> Self {
        Self {
            mode: value.mode.clone(),
            bind_host: value.bind_host.clone(),
            port: value.port,
            remote_base_url: value.remote_base_url.clone(),
        }
    }
}

impl From<PersistedRuntimeConfig> for EditableRuntimeConfig {
    fn from(value: PersistedRuntimeConfig) -> Self {
        Self {
            mode: value.mode,
            bind_host: value.bind_host,
            port: value.port,
            remote_base_url: value.remote_base_url,
        }
    }
}

/// 配置加载结果。
pub(crate) enum LoadedConfig {
    /// 文件不存在；首次启动保持未初始化。
    Missing,
    /// 文件存在但解析失败；保留原始内容作为可修复草稿。
    Invalid { raw: String },
    /// 文件存在且解析成功。
    Valid(EditableRuntimeConfig),
}

/// 配置文件的原子读写。
#[derive(Debug, Clone)]
pub(crate) struct ConfigStore {
    config_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedFirewallState {
    cleanup_port: u16,
}

impl ConfigStore {
    pub(crate) fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub(crate) fn load(&self) -> LoadedConfig {
        match fs::read_to_string(&self.config_path) {
            Ok(raw) => match serde_json::from_str::<PersistedRuntimeConfig>(&raw) {
                Ok(persisted) => LoadedConfig::Valid(persisted.into()),
                Err(_) => LoadedConfig::Invalid { raw },
            },
            Err(error) if error.kind() == ErrorKind::NotFound => LoadedConfig::Missing,
            Err(_) => LoadedConfig::Invalid { raw: String::new() },
        }
    }

    pub(crate) fn save(&self, config: &EditableRuntimeConfig) -> Result<(), ShellRuntimeError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|_| {
                ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法创建应用数据目录")
            })?;
        }
        let content = serde_json::to_vec_pretty(&PersistedRuntimeConfig::from(config))
            .map_err(|_| ShellRuntimeError::new(ERROR_CONFIG_UNAVAILABLE, "运行配置序列化失败"))?;
        let parent = self.config_path.parent().unwrap_or_else(|| Path::new("."));
        let temp = tempfile::NamedTempFile::new_in(parent).map_err(|_| {
            ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法写入临时配置文件")
        })?;
        fs::write(temp.path(), &content)
            .map_err(|_| ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法写入运行配置"))?;
        temp.persist(&self.config_path)
            .map_err(|_| ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法保存运行配置"))?;
        Ok(())
    }

    pub(crate) fn load_firewall_cleanup(&self) -> Option<u16> {
        let path = self.firewall_state_path();
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str::<PersistedFirewallState>(&content)
            .ok()
            .map(|state| state.cleanup_port)
    }

    pub(crate) fn save_firewall_cleanup(&self, port: u16) -> Result<(), ShellRuntimeError> {
        let path = self.firewall_state_path();
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|_| {
            ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法保存防火墙清理状态")
        })?;
        let content = serde_json::to_vec_pretty(&PersistedFirewallState { cleanup_port: port })
            .map_err(|_| {
                ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "防火墙清理状态序列化失败")
            })?;
        let temp = tempfile::NamedTempFile::new_in(parent).map_err(|_| {
            ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法写入防火墙清理状态")
        })?;
        fs::write(temp.path(), &content).map_err(|_| {
            ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法写入防火墙清理状态")
        })?;
        temp.persist(path).map_err(|_| {
            ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法保存防火墙清理状态")
        })?;
        Ok(())
    }

    pub(crate) fn clear_firewall_cleanup(&self) -> Result<(), ShellRuntimeError> {
        match fs::remove_file(self.firewall_state_path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(ShellRuntimeError::new(
                ERROR_STORAGE_UNAVAILABLE,
                "无法清理防火墙清理状态",
            )),
        }
    }

    fn firewall_state_path(&self) -> PathBuf {
        self.config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(FIREWALL_STATE_FILE_NAME)
    }
}

/// 校验通过后可供引擎启动 core 的完整配置。
pub(crate) struct PreparedConfig {
    pub(crate) app_config: AppConfig,
    pub(crate) normalized: EditableRuntimeConfig,
}

/// 本地存储路径，由应用数据目录派生。
#[derive(Debug, Clone)]
pub struct StoragePaths {
    pub app_data_dir: PathBuf,
    pub database_path: PathBuf,
    pub files_dir: PathBuf,
}

impl StoragePaths {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let database_path = app_data_dir.join(DATABASE_FILE_NAME);
        let files_dir = app_data_dir.join(FILES_DIR_NAME);
        Self {
            app_data_dir,
            database_path,
            files_dir,
        }
    }

    pub fn ensure_created(&self) -> Result<(), ShellRuntimeError> {
        fs::create_dir_all(&self.app_data_dir)
            .and_then(|_| fs::create_dir_all(&self.files_dir))
            .map_err(|_| ShellRuntimeError::new(ERROR_STORAGE_UNAVAILABLE, "无法创建本地存储目录"))
    }
}

/// Desktop 平台策略：self-hosted 只允许 loopback；server-mode 允许有效 IP 监听。
pub(crate) fn prepare_config(
    request: &EditableRuntimeConfig,
    storage_paths: &StoragePaths,
) -> Result<PreparedConfig, RuntimeConfigValidationResult> {
    let mut field_errors = BTreeMap::<String, Vec<String>>::new();

    let mode = match parse_mode(&request.mode) {
        Some(mode) => mode,
        None => {
            push_error(&mut field_errors, "mode", "运行模式无效");
            return Err(validation_result(field_errors, Some(request.clone())));
        }
    };

    let bind_host = request.bind_host.trim().to_owned();
    if mode == RuntimeMode::SelfHosted && !is_loopback_host(&bind_host) {
        push_error(
            &mut field_errors,
            "bindHost",
            "本机模式只允许使用 127.0.0.1 或 ::1 作为监听地址",
        );
    }

    let port = validate_port(request.port, mode, &mut field_errors);
    let remote_base_url = if mode.is_remote() {
        match normalize_remote_url(&request.remote_base_url) {
            Ok(value) => value,
            Err(message) => {
                push_error(&mut field_errors, "remoteBaseUrl", &message);
                request.remote_base_url.trim().to_owned()
            }
        }
    } else {
        request.remote_base_url.trim().to_owned()
    };

    if !field_errors.is_empty() {
        return Err(validation_result(field_errors, Some(request.clone())));
    }

    let port = port.expect("无字段错误时 port 必须已通过范围校验");
    let normalized = EditableRuntimeConfig {
        mode: mode_string(mode),
        bind_host: bind_host.clone(),
        port: i64::from(port),
        remote_base_url: remote_base_url.clone(),
    };
    let mut app_config = shared_config(mode, bind_host, port, remote_base_url);
    app_config.storage = StorageConfig {
        database_path: storage_paths.database_path.to_string_lossy().into_owned(),
        files_dir: storage_paths.files_dir.to_string_lossy().into_owned(),
        auto_migrate: true,
    };

    for issue in app_config.validation_issues() {
        let (field, message) = match issue.path.as_str() {
            "server.bind_host" => ("bindHost", "监听地址必须是有效的 IP 地址"),
            "server.port" => ("port", "端口必须是 1 到 65535 之间的整数"),
            "server.remote_base_url" => ("remoteBaseUrl", "远端服务地址必须使用 http 或 https"),
            _ => {
                return Err(validation_result(
                    BTreeMap::from([("mode".to_owned(), vec!["运行配置校验失败".to_owned()])]),
                    Some(request.clone()),
                ));
            }
        };
        push_error(&mut field_errors, field, message);
    }

    if !field_errors.is_empty() {
        return Err(validation_result(field_errors, Some(request.clone())));
    }

    Ok(PreparedConfig {
        app_config,
        normalized,
    })
}

fn shared_config(
    mode: RuntimeMode,
    bind_host: String,
    port: u16,
    remote_base_url: String,
) -> AppConfig {
    use winestock_shared::RuntimeMode as SharedMode;
    let shared_mode = match mode {
        RuntimeMode::ClientOnly => SharedMode::ClientOnly,
        RuntimeMode::SelfHosted => SharedMode::SelfHosted,
        RuntimeMode::ServerMode => SharedMode::ServerMode,
        RuntimeMode::ConnectToRemote => SharedMode::ConnectToRemote,
    };
    AppConfig {
        server: ServerConfig {
            mode: shared_mode,
            bind_host,
            port,
            auto_start_server: true,
            remote_base_url,
        },
        storage: StorageConfig {
            database_path: String::new(),
            files_dir: String::new(),
            auto_migrate: true,
        },
    }
}

fn validation_result(
    field_errors: BTreeMap<String, Vec<String>>,
    normalized_config: Option<EditableRuntimeConfig>,
) -> RuntimeConfigValidationResult {
    RuntimeConfigValidationResult {
        valid: field_errors.is_empty(),
        field_errors,
        normalized_config,
    }
}

fn push_error(errors: &mut BTreeMap<String, Vec<String>>, field: &str, message: &str) {
    errors
        .entry(field.to_owned())
        .or_default()
        .push(message.to_owned());
}

pub(crate) fn parse_mode(value: &str) -> Option<RuntimeMode> {
    match value {
        "client-only" => Some(RuntimeMode::ClientOnly),
        "self-hosted" => Some(RuntimeMode::SelfHosted),
        "server-mode" => Some(RuntimeMode::ServerMode),
        "connect-to-remote" => Some(RuntimeMode::ConnectToRemote),
        _ => None,
    }
}

pub(crate) fn mode_string(mode: RuntimeMode) -> String {
    match mode {
        RuntimeMode::ClientOnly => "client-only",
        RuntimeMode::SelfHosted => "self-hosted",
        RuntimeMode::ServerMode => "server-mode",
        RuntimeMode::ConnectToRemote => "connect-to-remote",
    }
    .to_owned()
}

fn validate_port(
    port: i64,
    mode: RuntimeMode,
    errors: &mut BTreeMap<String, Vec<String>>,
) -> Option<u16> {
    if port == 0 && mode != RuntimeMode::SelfHosted {
        push_error(errors, "port", "该模式必须使用 1 到 65535 之间的端口");
        return None;
    }
    if !(1..=65535).contains(&port) && port != 0 {
        push_error(errors, "port", "端口必须是 1 到 65535 之间的整数");
        return None;
    }
    u16::try_from(port).ok()
}

fn is_loopback_host(host: &str) -> bool {
    matches!(
        host.parse::<IpAddr>().ok(),
        Some(IpAddr::V4(address)) if address.is_loopback()
    ) || matches!(
        host.parse::<IpAddr>().ok(),
        Some(IpAddr::V6(address)) if address.is_loopback()
    )
}

/// 校验并规范化远端 URL：http/https、无凭据、无查询/hash。
fn normalize_remote_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("远端服务地址不能为空".to_owned());
    }
    let url = Url::parse(trimmed).map_err(|_| "远端服务地址必须是合法 URL".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("远端服务地址必须使用 http 或 https".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("远端服务地址不能包含用户凭据".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("远端服务地址不能包含查询参数或片段".to_owned());
    }
    let trimmed_path = url.path().trim_end_matches('/').to_owned();
    let mut normalized = url;
    normalized.set_path(&trimmed_path);
    Ok(normalized.to_string().trim_end_matches('/').to_owned())
}

pub(crate) fn config_draft_from_raw(raw: &str) -> EditableRuntimeConfig {
    serde_json::from_str::<PersistedRuntimeConfig>(raw)
        .map(EditableRuntimeConfig::from)
        .unwrap_or_else(|_| EditableRuntimeConfig::default_draft())
}
