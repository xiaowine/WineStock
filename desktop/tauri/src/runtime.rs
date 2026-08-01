//! Desktop 进程级运行状态、配置持久化与本地 Axum 生命周期。
//!
//! 本模块属于 `desktop/tauri` 壳，只通过 `winestock_core` 启动/停止共享服务，
//! 不复制 core 的业务实现，也不直接代理 HTTP 请求。

use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as AsyncMutex;
use url::Url;
use winestock_core::{
    start_local_service, LocalServiceRuntimeError, RunningLocalService, ServerStartError,
};
use winestock_shared::{AppConfig, RuntimeMode as SharedRuntimeMode, ServerConfig, StorageConfig};

use crate::contract::{
    self, desktop_capabilities, stopped_service, ApplyRuntimeConfigResult, EditableRuntimeConfig,
    RuntimeConfigValidationResult, RuntimeMode, RuntimeServiceSnapshot, RuntimeSnapshot,
    ShellRuntimeError, ERROR_CONFIG_INVALID, ERROR_CONFIG_UNAVAILABLE, ERROR_DATABASE_OPEN_FAILED,
    ERROR_PORT_IN_USE, ERROR_SERVICE_CRASHED, ERROR_SERVICE_START_FAILED,
    ERROR_STORAGE_UNAVAILABLE, ERROR_UNSUPPORTED_RUNTIME_MODE,
};

/// 事件名与 frontend `src/shell/tauri.ts` 保持一致。
pub const RUNTIME_STATE_CHANGED_EVENT: &str = "winestock-runtime-state-changed";
pub const APP_RESUMED_EVENT: &str = "winestock-app-resumed";

const CONFIG_FILE_NAME: &str = "config.json";
const DATABASE_FILE_NAME: &str = "winestock.sqlite";
const FILES_DIR_NAME: &str = "files";

/// 配置文件的稳定 JSON 结构；只持久化前端可编辑字段，存储路径由平台派生。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedRuntimeConfig {
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
enum LoadedConfig {
    /// 文件不存在；首次启动保持未初始化。
    Missing,
    /// 文件存在但解析失败；保留原始内容作为可修复草稿。
    Invalid { raw: String },
    /// 文件存在且解析成功。
    Valid(EditableRuntimeConfig),
}

/// 配置文件的原子读写。
#[derive(Debug, Clone)]
struct ConfigStore {
    config_path: PathBuf,
}

impl ConfigStore {
    fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    fn load(&self) -> LoadedConfig {
        match fs::read_to_string(&self.config_path) {
            Ok(raw) => match serde_json::from_str::<PersistedRuntimeConfig>(&raw) {
                Ok(persisted) => LoadedConfig::Valid(persisted.into()),
                Err(_) => LoadedConfig::Invalid { raw },
            },
            Err(error) if error.kind() == ErrorKind::NotFound => LoadedConfig::Missing,
            Err(_) => LoadedConfig::Invalid { raw: String::new() },
        }
    }

    fn save(&self, config: &EditableRuntimeConfig) -> Result<(), ShellRuntimeError> {
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
}

/// 校验通过后可供引擎启动 core 的完整配置。
struct PreparedConfig {
    app_config: AppConfig,
    normalized: EditableRuntimeConfig,
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

/// Desktop 平台策略：self-hosted 只允许 loopback；server-mode 尚未实现。
fn prepare_config(
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

    if mode == RuntimeMode::ServerMode {
        push_error(
            &mut field_errors,
            "mode",
            "当前 Desktop 版本尚未支持 server-mode",
        );
        return Err(validation_result(field_errors, Some(request.clone())));
    }

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

fn parse_mode(value: &str) -> Option<RuntimeMode> {
    match value {
        "client-only" => Some(RuntimeMode::ClientOnly),
        "self-hosted" => Some(RuntimeMode::SelfHosted),
        "server-mode" => Some(RuntimeMode::ServerMode),
        "connect-to-remote" => Some(RuntimeMode::ConnectToRemote),
        _ => None,
    }
}

fn mode_string(mode: RuntimeMode) -> String {
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

/// 本地服务启动成功后的只读信息；只包含可安全返回给前端的字段。
#[derive(Debug, Clone)]
struct LocalServiceDetails {
    bound_address: String,
    api_base_url: String,
    local_auth_exchange_token: Option<String>,
}

/// Desktop 进程级运行管理器：所有配置与服务变更在同一个 async mutex 内串行执行。
pub struct DesktopRuntimeManager {
    app: Option<AppHandle>,
    store: ConfigStore,
    storage_paths: StoragePaths,
    inner: AsyncMutex<ManagerInner>,
}

struct ManagerInner {
    snapshot: RuntimeSnapshot,
    service: Option<RunningLocalService>,
    initialized: bool,
    config_status: String,
}

impl DesktopRuntimeManager {
    /// 创建管理器，读取持久配置并派生平台存储路径；不启动服务、不写入任何文件。
    pub fn new(app: Option<AppHandle>, app_data_dir: PathBuf) -> Self {
        let storage_paths = StoragePaths::new(app_data_dir.clone());
        let _ = storage_paths.ensure_created();
        let store = ConfigStore::new(app_data_dir.join(CONFIG_FILE_NAME));
        let (snapshot, initialized, config_status) = match store.load() {
            LoadedConfig::Missing => (
                unconfigured_snapshot(EditableRuntimeConfig::default_draft()),
                false,
                "unconfigured".to_owned(),
            ),
            LoadedConfig::Invalid { raw } => (
                invalid_snapshot(config_draft_from_raw(&raw)),
                false,
                "invalid".to_owned(),
            ),
            LoadedConfig::Valid(config) => match prepare_config(&config, &storage_paths) {
                Ok(_) => (
                    configured_snapshot(config.clone(), stopped_service()),
                    true,
                    "configured".to_owned(),
                ),
                Err(_) => (invalid_snapshot(config), false, "invalid".to_owned()),
            },
        };
        Self {
            app,
            store,
            storage_paths,
            inner: AsyncMutex::new(ManagerInner {
                snapshot,
                service: None,
                initialized,
                config_status,
            }),
        }
    }

    /// 应用冷启动策略：已有有效本地配置自动启动，首次未初始化只发布 stopped 快照。
    pub async fn initialize(&self) {
        let (candidate, config_status) = {
            let inner = self.inner.lock().await;
            (inner.snapshot.config.clone(), inner.config_status.clone())
        };
        if config_status != "configured" {
            return;
        }
        let _ = self.start_or_apply(candidate).await;
    }

    /// 返回当前权威快照。
    pub async fn snapshot(&self) -> RuntimeSnapshot {
        self.inner.lock().await.snapshot.clone()
    }

    /// 校验候选配置；不产生持久化或启动副作用。
    pub async fn validate(&self, config: EditableRuntimeConfig) -> RuntimeConfigValidationResult {
        match prepare_config(&config, &self.storage_paths) {
            Ok(prepared) => RuntimeConfigValidationResult {
                valid: true,
                field_errors: BTreeMap::new(),
                normalized_config: Some(prepared.normalized),
            },
            Err(result) => result,
        }
    }

    /// 应用并持久化候选配置；失败时尽力恢复之前的运行状态。
    pub async fn apply(&self, config: EditableRuntimeConfig) -> ApplyRuntimeConfigResult {
        let validation = match prepare_config(&config, &self.storage_paths) {
            Ok(prepared) => RuntimeConfigValidationResult {
                valid: true,
                field_errors: BTreeMap::new(),
                normalized_config: Some(prepared.normalized),
            },
            Err(result) => result,
        };
        if !validation.valid {
            let previous = self.current_with_draft(config.clone()).await;
            return ApplyRuntimeConfigResult {
                valid: false,
                field_errors: validation.field_errors,
                applied: false,
                snapshot: previous,
                error: None,
            };
        }

        let mut candidate = validation
            .normalized_config
            .clone()
            .unwrap_or(config.clone());
        // 首次自托管不抢占默认端口；绑定成功后把真实端口作为正式配置持久化。
        if !self.inner.lock().await.initialized
            && parse_mode(&candidate.mode) == Some(RuntimeMode::SelfHosted)
        {
            candidate.port = 0;
        }
        let previous = self.snapshot().await;
        match self.start_or_apply(candidate.clone()).await {
            Ok(snapshot) => ApplyRuntimeConfigResult {
                valid: true,
                field_errors: BTreeMap::new(),
                applied: true,
                snapshot,
                error: None,
            },
            Err(result) => {
                let restored = self.restore_from_previous(previous).await;
                ApplyRuntimeConfigResult {
                    valid: true,
                    field_errors: BTreeMap::new(),
                    applied: false,
                    snapshot: restored,
                    error: result.error,
                }
            }
        }
    }

    /// 启动当前配置的本地服务；仅在 configured+local 状态可用。
    pub async fn start_local_service(&self) -> Result<RuntimeSnapshot, ShellRuntimeError> {
        let current = self.snapshot().await;
        if current.config_status != "configured" || current.service.ownership != "local" {
            return Err(ShellRuntimeError::new(
                ERROR_UNSUPPORTED_RUNTIME_MODE,
                "当前运行模式不提供本地服务",
            ));
        }
        match self.start_or_apply(current.config.clone()).await {
            Ok(snapshot) => Ok(snapshot),
            Err(result) => Err(result.error.unwrap_or_else(|| {
                ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务启动失败")
            })),
        }
    }

    /// 停止当前本地服务；配置本身保持有效。
    pub async fn stop_local_service(&self) -> Result<RuntimeSnapshot, ShellRuntimeError> {
        let mut inner = self.inner.lock().await;
        if inner.snapshot.service.ownership != "local" {
            return Err(ShellRuntimeError::new(
                ERROR_UNSUPPORTED_RUNTIME_MODE,
                "当前运行模式不提供本地服务",
            ));
        }
        if let Some(service) = inner.service.take() {
            service.shutdown().await.map_err(|_| {
                ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务停止失败")
            })?;
        }
        let mut snapshot = inner.snapshot.clone();
        snapshot.service = stopped_service();
        inner.snapshot = snapshot.clone();
        let result = snapshot.clone();
        drop(inner);
        self.emit(&result);
        Ok(result)
    }

    /// 重启当前本地服务；配置保持不变。
    pub async fn restart_local_service(&self) -> Result<RuntimeSnapshot, ShellRuntimeError> {
        let current = self.snapshot().await;
        if current.config_status != "configured" || current.service.ownership != "local" {
            return Err(ShellRuntimeError::new(
                ERROR_UNSUPPORTED_RUNTIME_MODE,
                "当前运行模式不提供本地服务",
            ));
        }
        self.stop_local_service().await?;
        self.start_or_apply(current.config.clone())
            .await
            .map_err(|result| {
                result.error.unwrap_or_else(|| {
                    ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务重启失败")
                })
            })
    }

    /// 应用配置并启动所需服务；错误结果包含应恢复的前一个快照。
    async fn start_or_apply(
        &self,
        config: EditableRuntimeConfig,
    ) -> Result<RuntimeSnapshot, ApplyRuntimeConfigResult> {
        let prepared = match prepare_config(&config, &self.storage_paths) {
            Ok(prepared) => prepared,
            Err(result) => {
                let snapshot = invalid_snapshot(config);
                return Err(ApplyRuntimeConfigResult {
                    valid: false,
                    field_errors: result.field_errors,
                    applied: false,
                    snapshot,
                    error: None,
                });
            }
        };

        let mode =
            parse_mode(&prepared.normalized.mode).expect("prepare_config 成功后 mode 必须已解析");
        let mut inner = self.inner.lock().await;

        if mode.is_remote() {
            if inner.snapshot.service.ownership == "local"
                && inner.snapshot.service.phase == "running"
            {
                if let Some(service) = inner.service.take() {
                    service
                        .shutdown()
                        .await
                        .map_err(|_| ApplyRuntimeConfigResult {
                            valid: true,
                            field_errors: BTreeMap::new(),
                            applied: false,
                            snapshot: inner.snapshot.clone(),
                            error: Some(ShellRuntimeError::new(
                                ERROR_SERVICE_START_FAILED,
                                "本地服务停止失败",
                            )),
                        })?;
                }
            }
            let snapshot = remote_snapshot(prepared.normalized.clone(), true);
            inner.initialized = true;
            inner.config_status = "configured".to_owned();
            inner.snapshot = snapshot.clone();
            self.store
                .save(&prepared.normalized)
                .map_err(|error| ApplyRuntimeConfigResult {
                    valid: true,
                    field_errors: BTreeMap::new(),
                    applied: false,
                    snapshot: inner.snapshot.clone(),
                    error: Some(error),
                })?;
            drop(inner);
            self.emit(&snapshot);
            return Ok(snapshot);
        }

        // 本地模式：替换正在运行的旧服务，避免端口/配置残留。
        if let Some(service) = inner.service.take() {
            service
                .shutdown()
                .await
                .map_err(|_| ApplyRuntimeConfigResult {
                    valid: true,
                    field_errors: BTreeMap::new(),
                    applied: false,
                    snapshot: inner.snapshot.clone(),
                    error: Some(ShellRuntimeError::new(
                        ERROR_SERVICE_START_FAILED,
                        "旧本地服务停止失败",
                    )),
                })?;
        }

        let mut starting = inner.snapshot.clone();
        starting.config = prepared.normalized.clone();
        starting.service = RuntimeServiceSnapshot {
            ownership: "local".to_owned(),
            phase: "starting".to_owned(),
            api_base_url: None,
            bound_address: None,
            local_auth_exchange_token: None,
            error: None,
        };
        inner.snapshot = starting.clone();
        drop(inner);
        self.emit(&starting);

        let started = self.start_service(&prepared.app_config).await;
        let mut inner = self.inner.lock().await;
        match started {
            Ok((service, details)) => {
                let effective = effective_config(&prepared.normalized, &details);
                // RunningLocalService 必须由 manager 独占持有；否则离开本函数即发送 shutdown，
                // 造成快照显示 running 而端口已释放。
                inner.service = Some(service);
                self.store
                    .save(&effective)
                    .map_err(|error| ApplyRuntimeConfigResult {
                        valid: true,
                        field_errors: BTreeMap::new(),
                        applied: false,
                        snapshot: inner.snapshot.clone(),
                        error: Some(error),
                    })?;
                inner.initialized = true;
                inner.config_status = "configured".to_owned();
                let snapshot = local_running_snapshot(effective, &details, true);
                inner.snapshot = snapshot.clone();
                drop(inner);
                self.emit(&snapshot);
                Ok(snapshot)
            }
            Err(error) => {
                let failed = failed_snapshot(prepared.normalized.clone(), error.clone(), true);
                inner.snapshot = failed.clone();
                drop(inner);
                self.emit(&failed);
                Err(ApplyRuntimeConfigResult {
                    valid: true,
                    field_errors: BTreeMap::new(),
                    applied: false,
                    snapshot: failed,
                    error: Some(error),
                })
            }
        }
    }

    /// 启动本地 Axum；端口占用时按策略自动改用动态端口重试一次。
    async fn start_service(
        &self,
        app_config: &AppConfig,
    ) -> Result<(RunningLocalService, LocalServiceDetails), ShellRuntimeError> {
        let mut config = app_config.clone();
        let first = start_local_service(&config).await;
        if let Err(LocalServiceRuntimeError::Server(ServerStartError::Bind { source, .. })) = &first
        {
            if source.kind() == std::io::ErrorKind::AddrInUse
                && config.server.mode == SharedRuntimeMode::SelfHosted
                && config.server.port != 0
            {
                config.server.port = 0;
                return map_start_result(start_local_service(&config).await);
            }
        }
        map_start_result(first)
    }

    /// 等待本地服务退出并停止；供应用退出流程使用。
    pub async fn shutdown_local_service(&self, timeout: Duration) {
        let mut inner = self.inner.lock().await;
        if let Some(service) = inner.service.take() {
            let _ = tokio::time::timeout(timeout, service.shutdown()).await;
        }
        let mut snapshot = inner.snapshot.clone();
        snapshot.service = stopped_service();
        inner.snapshot = snapshot.clone();
        drop(inner);
        self.emit(&snapshot);
    }

    /// 后台监视本地服务异常退出并发布 `service_crashed` 快照。
    pub fn spawn_monitor(this: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let (finished, ownership) = {
                    let inner = this.inner.lock().await;
                    (
                        inner
                            .service
                            .as_ref()
                            .is_some_and(RunningLocalService::is_finished),
                        inner.snapshot.service.ownership.clone(),
                    )
                };
                if finished && ownership == "local" {
                    let mut inner = this.inner.lock().await;
                    if let Some(service) = inner.service.take() {
                        let _ = service.wait().await;
                        let mut snapshot = inner.snapshot.clone();
                        snapshot.service = RuntimeServiceSnapshot {
                            ownership: "local".to_owned(),
                            phase: "failed".to_owned(),
                            api_base_url: None,
                            bound_address: None,
                            local_auth_exchange_token: None,
                            error: Some(ShellRuntimeError::new(
                                ERROR_SERVICE_CRASHED,
                                "本地服务意外退出",
                            )),
                        };
                        inner.snapshot = snapshot.clone();
                        drop(inner);
                        this.emit(&snapshot);
                    }
                }
            }
        });
    }

    async fn restore_from_previous(&self, previous: RuntimeSnapshot) -> RuntimeSnapshot {
        if previous.service.ownership == "local" && previous.service.phase == "running" {
            if let Ok(prepared) = prepare_config(&previous.config, &self.storage_paths) {
                match self.start_service(&prepared.app_config).await {
                    Ok((service, details)) => {
                        let restored = local_running_snapshot(
                            effective_config(&prepared.normalized, &details),
                            &details,
                            true,
                        );
                        let mut inner = self.inner.lock().await;
                        inner.service = Some(service);
                        inner.snapshot = restored.clone();
                        drop(inner);
                        self.emit(&restored);
                        return restored;
                    }
                    Err(error) => {
                        let failed = failed_snapshot(previous.config.clone(), error, true);
                        let mut inner = self.inner.lock().await;
                        inner.snapshot = failed.clone();
                        drop(inner);
                        self.emit(&failed);
                        return failed;
                    }
                }
            }
        }
        let mut inner = self.inner.lock().await;
        inner.snapshot = previous.clone();
        drop(inner);
        self.emit(&previous);
        previous
    }

    /// 返回当前快照，但把配置替换为给定草稿（用于校验失败时保留用户编辑）。
    async fn current_with_draft(&self, draft: EditableRuntimeConfig) -> RuntimeSnapshot {
        let inner = self.inner.lock().await;
        let mut snapshot = inner.snapshot.clone();
        snapshot.config = draft;
        snapshot
    }

    fn emit(&self, snapshot: &RuntimeSnapshot) {
        if let Some(app) = self.app.as_ref() {
            let _ = app.emit(RUNTIME_STATE_CHANGED_EVENT, snapshot);
        }
    }
}

/// 发布 app-resumed 事件；由窗口焦点事件调用。
pub fn emit_app_resumed(app: &AppHandle) {
    let _ = app.emit(APP_RESUMED_EVENT, ());
}

fn unconfigured_snapshot(config: EditableRuntimeConfig) -> RuntimeSnapshot {
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

fn invalid_snapshot(config: EditableRuntimeConfig) -> RuntimeSnapshot {
    let mut snapshot = unconfigured_snapshot(config);
    snapshot.config_status = "invalid".to_owned();
    snapshot.service.error = Some(ShellRuntimeError::new(
        ERROR_CONFIG_INVALID,
        "运行配置文件损坏或校验失败",
    ));
    snapshot
}

fn configured_snapshot(
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

fn remote_snapshot(config: EditableRuntimeConfig, _initialized: bool) -> RuntimeSnapshot {
    let mut snapshot = configured_snapshot(config.clone(), stopped_service());
    snapshot.service = RuntimeServiceSnapshot {
        ownership: "remote".to_owned(),
        phase: "stopped".to_owned(),
        api_base_url: Some(config.remote_base_url.clone()),
        bound_address: None,
        local_auth_exchange_token: None,
        error: None,
    };
    snapshot.capabilities = desktop_capabilities(true, &snapshot.service.ownership);
    snapshot
}

fn local_running_snapshot(
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
        error: None,
    };
    snapshot
}

fn failed_snapshot(
    config: EditableRuntimeConfig,
    error: ShellRuntimeError,
    _initialized: bool,
) -> RuntimeSnapshot {
    let mut snapshot = configured_snapshot(config.clone(), stopped_service());
    snapshot.service = RuntimeServiceSnapshot {
        ownership: "local".to_owned(),
        phase: "failed".to_owned(),
        api_base_url: None,
        bound_address: None,
        local_auth_exchange_token: None,
        error: Some(error),
    };
    snapshot
}

fn effective_config(
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

fn config_draft_from_raw(raw: &str) -> EditableRuntimeConfig {
    serde_json::from_str::<PersistedRuntimeConfig>(raw)
        .map(EditableRuntimeConfig::from)
        .unwrap_or_else(|_| EditableRuntimeConfig::default_draft())
}

fn map_start_result(
    result: Result<RunningLocalService, LocalServiceRuntimeError>,
) -> Result<(RunningLocalService, LocalServiceDetails), ShellRuntimeError> {
    match result {
        Ok(service) => {
            let info = service.info().clone();
            let port = info.bound_addr.port();
            Ok((
                service,
                LocalServiceDetails {
                    bound_address: info.bound_addr.to_string(),
                    api_base_url: format!("http://127.0.0.1:{port}"),
                    local_auth_exchange_token: info
                        .local_session_token
                        .as_ref()
                        .map(|token| token.expose().to_owned()),
                },
            ))
        }
        Err(error) => Err(map_service_error(&error)),
    }
}

fn map_service_error(error: &LocalServiceRuntimeError) -> ShellRuntimeError {
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
