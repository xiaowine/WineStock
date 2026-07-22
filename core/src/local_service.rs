//! core 本地 Axum 服务的统一运行句柄。
//!
//! 本模块属于 `core axum library` 层，把 bootstrap、端口绑定、serve task 和 graceful shutdown
//! 收敛为平台无关 API。平台 shell 决定调用时机与超时策略，本模块不监听进程或 Activity 生命周期。

use std::{error::Error, fmt, net::SocketAddr, path::PathBuf};

use tokio::{sync::oneshot, task::JoinHandle};
use winestock_shared::AppConfig;

use crate::{bind_server, bootstrap_from_config, CoreBootstrapError, ServerStartError};

/// 本地服务成功启动后可供平台展示或诊断的只读信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalServiceInfo {
    /// 操作系统实际绑定的地址；配置端口为 0 时包含分配后的真实端口。
    pub bound_addr: SocketAddr,

    /// core 实际打开的 SQLite 文件路径。
    pub database_path: PathBuf,

    /// core 实际使用的大对象文件目录。
    pub files_dir: PathBuf,

    /// 当前数据库是否仍需要创建首个管理员。
    pub admin_setup_required: bool,
}

/// 本地服务启动、运行或等待结束时的统一错误。
#[derive(Debug)]
pub enum LocalServiceRuntimeError {
    /// core 数据库、migration、权限或业务默认数据初始化失败。
    Bootstrap(CoreBootstrapError),

    /// bootstrap 没有返回当前本地模式必须存在的运行状态。
    LocalServiceNotInitialized,

    /// TCP 绑定或 Axum serve 失败。
    Server(ServerStartError),

    /// Tokio serve task panic 或被异常取消。
    Task(tokio::task::JoinError),
}

impl fmt::Display for LocalServiceRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bootstrap(source) => write!(f, "failed to bootstrap local service: {source}"),
            Self::LocalServiceNotInitialized => {
                write!(f, "core did not initialize required local service state")
            }
            Self::Server(source) => write!(f, "local service runtime failed: {source}"),
            Self::Task(_) => write!(f, "local service task stopped unexpectedly"),
        }
    }
}

impl Error for LocalServiceRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bootstrap(source) => Some(source),
            Self::Server(source) => Some(source),
            Self::Task(source) => Some(source),
            Self::LocalServiceNotInitialized => None,
        }
    }
}

/// 已启动且由单个平台 shell 独占持有的本地 Axum 服务。
#[derive(Debug)]
pub struct RunningLocalService {
    info: LocalServiceInfo,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), ServerStartError>>,
}

impl RunningLocalService {
    /// 返回服务启动时冻结的实际地址和存储信息。
    pub fn info(&self) -> &LocalServiceInfo {
        &self.info
    }

    /// 返回 Axum serve task 是否已经结束；平台可据此发现异常退出。
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    /// 请求 graceful shutdown 并等待 Axum task 完整结束。
    pub async fn shutdown(self) -> Result<(), LocalServiceRuntimeError> {
        let RunningLocalService {
            shutdown,
            task,
            info: _,
        } = self;
        let _ = shutdown.send(());
        join_service_task(task).await
    }

    /// 等待服务自行结束，用于平台监控意外 serve 错误。
    pub async fn wait(self) -> Result<(), LocalServiceRuntimeError> {
        let RunningLocalService {
            shutdown,
            task,
            info: _,
        } = self;
        // 等待期间保留 sender，避免仅因进入 wait 就触发 graceful shutdown。
        let shutdown_guard = shutdown;
        let result = join_service_task(task).await;
        drop(shutdown_guard);
        result
    }
}

/// 按共享配置绑定端口、初始化 core 并启动可停止的本地 Axum 服务。
pub async fn start_local_service(
    config: &AppConfig,
) -> Result<RunningLocalService, LocalServiceRuntimeError> {
    // 先绑定端口，使端口冲突在数据库 migration 等有副作用的初始化之前失败。
    let bound = bind_server(&config.server)
        .await
        .map_err(LocalServiceRuntimeError::Server)?;
    let bound_addr = bound.bound_addr();
    let bootstrap = bootstrap_from_config(config)
        .await
        .map_err(LocalServiceRuntimeError::Bootstrap)?;
    let local = bootstrap
        .local_service
        .ok_or(LocalServiceRuntimeError::LocalServiceNotInitialized)?;
    let info = LocalServiceInfo {
        bound_addr,
        database_path: local.storage.database_path.clone(),
        files_dir: local.storage.files_dir.clone(),
        admin_setup_required: local.auth.admin_setup_required,
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        bound
            .serve_local_with_shutdown(&local, async move {
                let _ = shutdown_rx.await;
            })
            .await
    });

    Ok(RunningLocalService {
        info,
        shutdown: shutdown_tx,
        task,
    })
}

async fn join_service_task(
    task: JoinHandle<Result<(), ServerStartError>>,
) -> Result<(), LocalServiceRuntimeError> {
    task.await
        .map_err(LocalServiceRuntimeError::Task)?
        .map_err(LocalServiceRuntimeError::Server)
}

#[cfg(test)]
#[path = "tests/local_service.rs"]
mod tests;
