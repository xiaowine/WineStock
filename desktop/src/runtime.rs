//! Desktop 进程级运行状态、配置持久化与本地 Axum 生命周期。
//!
//! 本模块只保留 DesktopRuntimeManager 的生命周期编排；配置、快照和错误映射分别位于同级子模块。

use crate::contract::{
    self, stopped_service, ApplyRuntimeConfigResult, EditableRuntimeConfig,
    RuntimeConfigValidationResult, RuntimeMode, RuntimeServiceSnapshot, RuntimeSnapshot,
    ShellRuntimeError, ERROR_SERVICE_CRASHED, ERROR_SERVICE_START_FAILED,
    ERROR_UNSUPPORTED_RUNTIME_MODE,
};
use crate::firewall;
use crate::lan_access::discover_lan_access_urls;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;
use winestock_core::{
    start_local_service, LocalServiceRuntimeError, RunningLocalService, ServerStartError,
};
use winestock_shared::{AppConfig, RuntimeMode as SharedRuntimeMode};
#[path = "runtime_config.rs"]
mod config;
#[path = "runtime_snapshot.rs"]
mod snapshot;
use config::{
    config_draft_from_raw, parse_mode, prepare_config, ConfigStore, LoadedConfig, StoragePaths,
};
#[allow(unused_imports)]
pub(crate) use snapshot::local_api_base_url;
use snapshot::{
    apply_cleanup_status, configured_snapshot, effective_config, failed_snapshot,
    firewall_snapshot_for_error, invalid_snapshot, local_running_snapshot, map_start_result,
    remote_snapshot, unconfigured_snapshot, LocalServiceDetails,
};

pub const RUNTIME_STATE_CHANGED_EVENT: &str = "winestock-runtime-state-changed";
pub const APP_RESUMED_EVENT: &str = "winestock-app-resumed";
const CONFIG_FILE_NAME: &str = "config.json";

fn is_firewall_error(error: &ShellRuntimeError) -> bool {
    matches!(
        error.code.as_str(),
        contract::ERROR_FIREWALL_AUTHORIZATION_REQUIRED
            | contract::ERROR_FIREWALL_POLICY_BLOCKED
            | contract::ERROR_FIREWALL_PROFILE_UNSUPPORTED
            | contract::ERROR_FIREWALL_SERVICE_UNAVAILABLE
            | contract::ERROR_FIREWALL_RULE_UPDATE_FAILED
            | contract::ERROR_FIREWALL_CLEANUP_PENDING
    )
}

fn firewall_cleanup_snapshot(port: u16) -> crate::contract::RuntimeFirewallSnapshot {
    crate::contract::RuntimeFirewallSnapshot {
        status: "cleanup-pending".to_owned(),
        port: Some(port),
        scope: Some("local-subnet".to_owned()),
    }
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
        let (mut snapshot, initialized, config_status) = match store.load() {
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
        if let Some(port) = store.load_firewall_cleanup() {
            apply_cleanup_status(&mut snapshot, Some(firewall_cleanup_snapshot(port)));
        }
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
        let _ = self.start_or_apply(candidate, false).await;
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
        match self.start_or_apply(candidate.clone(), true).await {
            Ok(snapshot) => ApplyRuntimeConfigResult {
                valid: true,
                field_errors: BTreeMap::new(),
                applied: true,
                error: snapshot.service.error.clone(),
                snapshot,
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
        match self.start_or_apply(current.config.clone(), true).await {
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
        let (shutdown_error, cleanup) = self.stop_owned_service(&mut inner, false).await;
        let mut snapshot = inner.snapshot.clone();
        snapshot.service = stopped_service();
        apply_cleanup_status(&mut snapshot, cleanup);
        if let Some(error) = shutdown_error {
            snapshot.service.error = Some(error.clone());
            inner.snapshot = snapshot.clone();
            drop(inner);
            self.emit(&snapshot);
            return Err(error);
        }
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
        self.start_or_apply(current.config.clone(), true)
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
        configure_firewall: bool,
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
            let pending_cleanup = inner
                .snapshot
                .service
                .firewall
                .as_ref()
                .filter(|firewall| firewall.status == "cleanup-pending")
                .cloned();
            let previous_mode = inner.snapshot.config.mode.clone();
            let mut cleanup = pending_cleanup.clone();
            if inner.snapshot.service.ownership == "local" {
                let (shutdown_error, removed) = self.stop_owned_service(&mut inner, true).await;
                if let Some(error) = shutdown_error {
                    return Err(ApplyRuntimeConfigResult {
                        valid: true,
                        field_errors: BTreeMap::new(),
                        applied: false,
                        snapshot: inner.snapshot.clone(),
                        error: Some(error),
                    });
                }
                cleanup = if previous_mode == "server-mode" {
                    removed
                } else {
                    pending_cleanup
                };
            }
            let mut snapshot = remote_snapshot(prepared.normalized.clone(), true);
            apply_cleanup_status(&mut snapshot, cleanup);

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

        // 本地模式：替换正在运行的旧服务，避免端口/配置残留。只有离开 server-mode
        // 才撤销持久化的防火墙规则；server-mode 内部重启由新的 ensure 收敛端口规则。
        let remove_firewall_rule = inner.snapshot.config.mode == "server-mode"
            && prepared.normalized.mode != "server-mode";
        let (shutdown_error, cleanup) = self
            .stop_owned_service(&mut inner, remove_firewall_rule)
            .await;
        if let Some(error) = shutdown_error {
            return Err(ApplyRuntimeConfigResult {
                valid: true,
                field_errors: BTreeMap::new(),
                applied: false,
                snapshot: inner.snapshot.clone(),
                error: Some(error),
            });
        }

        let mut starting = inner.snapshot.clone();
        starting.config = prepared.normalized.clone();
        starting.service = RuntimeServiceSnapshot {
            ownership: "local".to_owned(),
            phase: "starting".to_owned(),
            api_base_url: None,
            bound_address: None,
            local_auth_exchange_token: None,
            lan_access_urls: None,
            firewall: None,
            error: None,
        };
        inner.snapshot = starting.clone();
        drop(inner);
        self.emit(&starting);

        let started = self
            .start_service(
                &prepared.app_config,
                &prepared.normalized,
                configure_firewall,
            )
            .await;
        let mut inner = self.inner.lock().await;
        match started {
            Ok((service, details)) => {
                let effective = effective_config(&prepared.normalized, &details);
                // RunningLocalService 必须由 manager 独占持有；否则离开本函数即发送 shutdown，
                // 造成快照显示 running 而端口已释放。
                inner.service = Some(service);
                if let Err(error) = self.store.save(&effective) {
                    // 配置落盘失败时不能留下仍在监听的服务句柄，否则恢复旧快照后会出现
                    // “显示未运行但端口仍被占用”的状态；先停止新服务，再把错误交给调用方恢复。
                    if let Some(service) = inner.service.take() {
                        let _ = service.shutdown().await;
                    }
                    return Err(ApplyRuntimeConfigResult {
                        valid: true,
                        field_errors: BTreeMap::new(),
                        applied: false,
                        snapshot: inner.snapshot.clone(),
                        error: Some(error),
                    });
                }
                inner.initialized = true;
                inner.config_status = "configured".to_owned();
                let mut snapshot = local_running_snapshot(effective, &details, true);
                apply_cleanup_status(&mut snapshot, cleanup.clone());
                inner.snapshot = snapshot.clone();
                drop(inner);
                self.emit(&snapshot);
                Ok(snapshot)
            }
            Err(error) => {
                let mut failed = failed_snapshot(prepared.normalized.clone(), error.clone(), true);
                apply_cleanup_status(&mut failed, cleanup);
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

    /// 启动本地 Axum；仅 self-hosted 的端口占用允许改用动态端口重试一次。
    async fn start_service(
        &self,
        app_config: &AppConfig,
        runtime_config: &EditableRuntimeConfig,
        configure_firewall: bool,
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
                return map_start_result(start_local_service(&config).await, false);
            }
        }
        let server_mode = config.server.mode == SharedRuntimeMode::ServerMode;
        let started = map_start_result(first, server_mode)?;
        if !server_mode {
            return Ok(started);
        }
        let (service, mut details) = started;
        let port = service.info().bound_addr.port();
        let firewall_result = if configure_firewall {
            firewall::ensure(port, self.app.is_some())
        } else {
            firewall::probe(port, self.app.is_some())
        };
        match firewall_result {
            Ok(status) => {
                if status.status == "ready" {
                    let _ = self.store.clear_firewall_cleanup();
                }
                details.firewall = Some(status)
            }
            Err(error) => {
                details.firewall = firewall_snapshot_for_error(runtime_config, &error);
                details.firewall_error = Some(error);
            }
        }
        Ok((service, details))
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

    /// 显式修复当前 server-mode 的防火墙规则，或重试离开 server-mode 时未完成的清理。
    pub async fn repair_firewall(&self) -> Result<RuntimeSnapshot, ShellRuntimeError> {
        let current = self.snapshot().await;
        let cleanup_pending = current
            .service
            .firewall
            .as_ref()
            .is_some_and(|firewall| firewall.status == "cleanup-pending");

        if current.config.mode == "server-mode" {
            let port = u16::try_from(current.config.port).map_err(|_| {
                ShellRuntimeError::new(
                    contract::ERROR_FIREWALL_RULE_UPDATE_FAILED,
                    "当前 server-mode 端口无效",
                )
            })?;
            let result = firewall::ensure(port, self.app.is_some());
            let mut inner = self.inner.lock().await;
            match result {
                Ok(status) => {
                    if status.status == "ready" {
                        let _ = self.store.clear_firewall_cleanup();
                    }
                    inner.snapshot.service.firewall = Some(status);
                    if inner
                        .snapshot
                        .service
                        .error
                        .as_ref()
                        .is_some_and(is_firewall_error)
                    {
                        inner.snapshot.service.error = None;
                    }
                }
                Err(error) => {
                    inner.snapshot.service.firewall =
                        firewall_snapshot_for_error(&inner.snapshot.config, &error);
                    inner.snapshot.service.error = Some(error.clone());
                    let snapshot = inner.snapshot.clone();
                    drop(inner);
                    self.emit(&snapshot);
                    return Err(error);
                }
            }
            let snapshot = inner.snapshot.clone();
            drop(inner);
            self.emit(&snapshot);
            return Ok(snapshot);
        }

        if !cleanup_pending {
            return Err(ShellRuntimeError::new(
                ERROR_UNSUPPORTED_RUNTIME_MODE,
                "当前运行模式没有待清理的防火墙规则",
            ));
        }
        let port = current
            .service
            .firewall
            .as_ref()
            .and_then(|firewall| firewall.port)
            .ok_or_else(|| {
                ShellRuntimeError::new(
                    contract::ERROR_FIREWALL_CLEANUP_PENDING,
                    "待清理的 Windows 防火墙规则缺少端口信息",
                )
            })?;
        firewall::remove(port, self.app.is_some())?;
        let _ = self.store.clear_firewall_cleanup();
        let mut inner = self.inner.lock().await;
        inner.snapshot.service.firewall = None;
        if inner
            .snapshot
            .service
            .error
            .as_ref()
            .is_some_and(|error| error.code == contract::ERROR_FIREWALL_CLEANUP_PENDING)
        {
            inner.snapshot.service.error = None;
        }
        let snapshot = inner.snapshot.clone();
        drop(inner);
        self.emit(&snapshot);
        Ok(snapshot)
    }

    /// 重新读取 server-mode 的真实网卡地址并发布最新快照。
    ///
    /// Desktop 获得焦点时调用此方法，以覆盖 Wi-Fi、VPN 或网卡启停造成的地址变化；
    /// 服务句柄和本机 API 地址保持不变，地址发现失败只表现为空列表，不影响服务运行。
    pub async fn refresh_network_state(&self) {
        let mut inner = self.inner.lock().await;
        if inner.snapshot.service.ownership != "local"
            || inner.snapshot.service.phase != "running"
            || parse_mode(&inner.snapshot.config.mode) != Some(RuntimeMode::ServerMode)
        {
            return;
        }

        let Some(service) = inner.service.as_ref() else {
            return;
        };
        let port = service.info().bound_addr.port();
        let urls = discover_lan_access_urls(service.info().bound_addr);
        match firewall::probe(port, self.app.is_some()) {
            Ok(status) => {
                if status.status == "ready" {
                    let _ = self.store.clear_firewall_cleanup();
                }
                inner.snapshot.service.firewall = Some(status);
                if inner
                    .snapshot
                    .service
                    .error
                    .as_ref()
                    .is_some_and(is_firewall_error)
                {
                    inner.snapshot.service.error = None;
                }
            }
            Err(error) => {
                inner.snapshot.service.firewall =
                    firewall_snapshot_for_error(&inner.snapshot.config, &error);
                inner.snapshot.service.error = Some(error);
            }
        }
        inner.snapshot.service.lan_access_urls = Some(urls);
        let snapshot = inner.snapshot.clone();
        drop(inner);
        self.emit(&snapshot);
    }

    async fn stop_owned_service(
        &self,
        inner: &mut ManagerInner,
        remove_firewall_rule: bool,
    ) -> (
        Option<ShellRuntimeError>,
        Option<crate::contract::RuntimeFirewallSnapshot>,
    ) {
        let previous = inner.snapshot.clone();
        let service_port = inner
            .service
            .as_ref()
            .map(|service| service.info().bound_addr.port());
        let shutdown_error = if let Some(service) = inner.service.take() {
            service
                .shutdown()
                .await
                .err()
                .map(|_| ShellRuntimeError::new(ERROR_SERVICE_START_FAILED, "本地服务停止失败"))
        } else {
            None
        };
        let cleanup = if remove_firewall_rule {
            self.cleanup_firewall(&previous, service_port)
        } else {
            None
        };
        (shutdown_error, cleanup)
    }

    fn cleanup_firewall(
        &self,
        previous: &RuntimeSnapshot,
        service_port: Option<u16>,
    ) -> Option<crate::contract::RuntimeFirewallSnapshot> {
        if previous.config.mode != "server-mode" || previous.service.ownership != "local" {
            return None;
        }
        let port = previous
            .service
            .firewall
            .as_ref()
            .and_then(|firewall| firewall.port)
            .or(service_port)
            .or_else(|| u16::try_from(previous.config.port).ok());
        let Some(port) = port else { return None };
        match firewall::remove(port, self.app.is_some()) {
            Ok(()) => {
                let _ = self.store.clear_firewall_cleanup();
                None
            }
            Err(_) => {
                let _ = self.store.save_firewall_cleanup(port);
                Some(firewall_cleanup_snapshot(port))
            }
        }
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
                            lan_access_urls: None,
                            firewall: None,
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
                match self
                    .start_service(&prepared.app_config, &prepared.normalized, false)
                    .await
                {
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
    let manager = app.state::<Arc<DesktopRuntimeManager>>().inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        manager.refresh_network_state().await;
        let _ = app.emit(APP_RESUMED_EVENT, ());
    });
}

#[cfg(test)]
mod tests {
    use super::local_api_base_url;
    use std::net::SocketAddr;

    #[test]
    fn local_api_url_uses_loopback_for_wildcard_and_bound_ip_for_specific_address() {
        assert_eq!(
            local_api_base_url("0.0.0.0:17890".parse::<SocketAddr>().unwrap()),
            "http://127.0.0.1:17890"
        );
        assert_eq!(
            local_api_base_url("192.168.1.20:17890".parse::<SocketAddr>().unwrap()),
            "http://192.168.1.20:17890"
        );
        assert_eq!(
            local_api_base_url("[::]:17890".parse::<SocketAddr>().unwrap()),
            "http://[::1]:17890"
        );
        assert_eq!(
            local_api_base_url("[fd00::20]:17890".parse::<SocketAddr>().unwrap()),
            "http://[fd00::20]:17890"
        );
    }
}
