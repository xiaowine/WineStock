//! Android 进程内长期 Tokio Runtime 与本地 core 句柄。
//!
//! Kotlin 通过单线程 executor 串行调用本模块；这里不持有 JVM、Activity 或 WebView 引用。

use tokio::runtime::{Builder, Runtime};
use winestock_core::{start_local_service, LocalServiceInfo, RunningLocalService};
use winestock_shared::AppConfig;

use crate::{contract::NativeServiceState, error::NativeError};

/// Android 应用进程中唯一的 Rust 运行引擎。
pub struct NativeEngine {
    runtime: Runtime,
    service: Option<RunningLocalService>,
    last_error: Option<NativeError>,
}

impl NativeEngine {
    /// 创建长期 multi-thread Tokio Runtime；调用方必须位于非 async 后台线程。
    pub fn new() -> Result<Self, NativeError> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("winestock-core")
            .enable_all()
            .build()
            .map_err(|_| NativeError::engine_unavailable())?;
        Ok(Self {
            runtime,
            service: None,
            last_error: None,
        })
    }

    /// 幂等启动本地服务；已经运行时直接返回当前状态。
    pub fn start(&mut self, config: &AppConfig) -> Result<NativeServiceState, NativeError> {
        self.refresh_finished_service();
        if let Some(service) = self.service.as_ref() {
            return Ok(running_state(service.info()));
        }

        self.last_error = None;
        let service = self
            .runtime
            .block_on(start_local_service(config))
            .map_err(NativeError::from)?;
        let state = running_state(service.info());
        self.service = Some(service);
        Ok(state)
    }

    /// 幂等停止本地服务并等待 core graceful shutdown。
    pub fn stop(&mut self) -> Result<NativeServiceState, NativeError> {
        self.refresh_finished_service();
        if let Some(service) = self.service.take() {
            self.runtime
                .block_on(service.shutdown())
                .map_err(NativeError::from)?;
        }
        self.last_error = None;
        Ok(NativeServiceState::stopped())
    }

    /// 使用候选配置停止并重新启动本地服务。
    pub fn restart(&mut self, config: &AppConfig) -> Result<NativeServiceState, NativeError> {
        self.stop()?;
        self.start(config)
    }

    /// 查询当前状态，并把已经结束的 serve task 转换成 failed。
    pub fn state(&mut self) -> NativeServiceState {
        self.refresh_finished_service();
        if let Some(service) = self.service.as_ref() {
            return running_state(service.info());
        }
        match self.last_error.clone() {
            Some(error) => NativeServiceState::failed(error),
            None => NativeServiceState::stopped(),
        }
    }

    /// engine 销毁前停止服务；Runtime 随随后对象 drop 在 JNI executor 线程关闭。
    pub fn shutdown(&mut self) -> Result<(), NativeError> {
        self.stop().map(|_| ())
    }

    fn refresh_finished_service(&mut self) {
        let is_finished = self
            .service
            .as_ref()
            .is_some_and(RunningLocalService::is_finished);
        if !is_finished {
            return;
        }

        let service = self.service.take().expect("已确认存在已结束服务");
        self.last_error = self
            .runtime
            .block_on(service.wait())
            .err()
            .map(NativeError::from);
        if self.last_error.is_none() {
            self.last_error = Some(NativeError::new(
                "service_crashed",
                "本地服务在未收到停止命令时结束",
            ));
        }
    }
}

fn running_state(info: &LocalServiceInfo) -> NativeServiceState {
    let port = info.bound_addr.port();
    NativeServiceState {
        phase: "running".to_owned(),
        bound_address: Some(info.bound_addr.to_string()),
        api_base_url: Some(format!("http://127.0.0.1:{port}")),
        admin_setup_required: Some(info.admin_setup_required),
        local_auth_exchange_token: info
            .local_session_token
            .as_ref()
            .map(|token| token.expose().to_owned()),
        error: None,
    }
}
