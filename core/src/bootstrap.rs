//! core 启动编排入口。
//!
//! 本模块属于 `core axum library` 层，负责根据平台壳传入的共享配置准备本地服务依赖。
//! 它不查找配置文件、不创建平台目录，也不处理服务端进程生命周期。

use std::{error::Error, fmt};

use winestock_shared::{AppConfig, RuntimeMode};

use crate::{
    auth::{bootstrap_auth, AuthBootstrap, AuthBootstrapError},
    external::{ExternalCatalogBootstrapError, ExternalCatalogRuntime},
    files::cleanup_orphaned_images,
    persistence::{
        migrate_storage_schema, open_sqlite_storage, StorageBootstrapError, StorageRuntime,
    },
    rbac::{bootstrap_builtin_rbac, RbacBootstrapError},
    security::random_urlsafe,
    stock::{bootstrap_default_templates, StockBootstrapError},
    FileCleanupError,
};

/// self-hosted 模式下每次启动生成的本机会话换取凭据。
///
/// 凭据只存进程内存，经 `LocalServiceInfo` 交给平台壳，再由壳内可信桥交给前端；
/// 它不落盘、不进入日志，比较时使用哈希摘要避免时序泄露。
#[derive(Clone, PartialEq, Eq)]
pub struct LocalSessionSecret(String);

impl LocalSessionSecret {
    /// 生成一次性的高熵换取凭据。
    pub(crate) fn generate() -> Result<Self, getrandom::Error> {
        Ok(Self(random_urlsafe(32)?))
    }

    /// 返回凭据明文；仅平台壳桥接和换取比较使用，不得写入日志。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for LocalSessionSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LocalSessionSecret")
            .field(&"[redacted]")
            .finish()
    }
}

/// core 根据启动配置完成的初始化结果。
#[derive(Debug, Clone)]
pub struct CoreBootstrap {
    /// 需要本地服务时包含启动结果；远端客户端模式下为空。
    pub local_service: Option<LocalServiceBootstrap>,
}

impl CoreBootstrap {
    /// 返回本次配置是否实际初始化了本地服务依赖。
    pub fn initialized_local_service(&self) -> bool {
        self.local_service.is_some()
    }
}

/// 本地 Axum 服务启动前必须准备好的共享状态。
#[derive(Debug, Clone)]
pub struct LocalServiceBootstrap {
    /// 本地存储运行时状态和 SeaORM 连接。
    pub storage: StorageRuntime,

    /// 鉴权启动结果，包括数据库托管设置和签名密钥。
    pub auth: AuthBootstrap,

    /// 外部商品资料查询 client，不包含第三方 Cookie 或会话。
    pub(crate) external_catalog: ExternalCatalogRuntime,

    /// self-hosted 模式的本机会话换取凭据；其它模式为空，端点随之不可用。
    pub(crate) local_session: Option<LocalSessionSecret>,
}

/// core 启动配置初始化错误。
#[derive(Debug)]
pub enum CoreBootstrapError {
    /// 本地存储打开、配置或迁移失败。
    Storage(StorageBootstrapError),

    /// 鉴权设置或签名密钥初始化失败。
    Auth(AuthBootstrapError),

    /// 内置权限初始化失败。
    Rbac(RbacBootstrapError),

    /// 库存默认模板初始化失败。
    Stock(StockBootstrapError),

    /// 临时图片孤儿文件清理失败。
    Files(FileCleanupError),

    /// 外部商品资料 HTTP client 初始化失败。
    ExternalCatalog(ExternalCatalogBootstrapError),
}

impl fmt::Display for CoreBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(source) => write!(f, "{source}"),
            Self::Auth(source) => write!(f, "{source}"),
            Self::Rbac(source) => write!(f, "{source}"),
            Self::Stock(source) => write!(f, "{source}"),
            Self::Files(source) => write!(f, "{source}"),
            Self::ExternalCatalog(source) => write!(f, "{source}"),
        }
    }
}

impl Error for CoreBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(source) => Some(source),
            Self::Auth(source) => Some(source),
            Self::Rbac(source) => Some(source),
            Self::Stock(source) => Some(source),
            Self::Files(source) => Some(source),
            Self::ExternalCatalog(source) => Some(source),
        }
    }
}

/// 使用已解析配置初始化 core，本函数不查找或读取配置文件。
pub async fn bootstrap_from_config(
    config: &AppConfig,
) -> Result<CoreBootstrap, CoreBootstrapError> {
    if !config.server.uses_local_service() {
        return Ok(CoreBootstrap {
            local_service: None,
        });
    }

    let storage = open_sqlite_storage(&config.storage)
        .await
        .map_err(CoreBootstrapError::Storage)?;

    if config.storage.auto_migrate {
        migrate_storage_schema(&storage)
            .await
            .map_err(CoreBootstrapError::Storage)?;
    }

    bootstrap_builtin_rbac(&storage.database)
        .await
        .map_err(CoreBootstrapError::Rbac)?;

    bootstrap_default_templates(&storage.database)
        .await
        .map_err(CoreBootstrapError::Stock)?;

    // migration 完成后清理超期临时图片，避免中断上传长期占用磁盘。
    cleanup_orphaned_images(&storage)
        .await
        .map_err(CoreBootstrapError::Files)?;

    let auth = bootstrap_auth(&storage.database)
        .await
        .map_err(CoreBootstrapError::Auth)?;

    let external_catalog =
        ExternalCatalogRuntime::build().map_err(CoreBootstrapError::ExternalCatalog)?;

    // 换取凭据仅面向"本机 UI + 可信壳"的 self-hosted 模式；server-mode 面向多客户端，保持登录。
    let local_session = if config.server.mode == RuntimeMode::SelfHosted {
        Some(
            LocalSessionSecret::generate()
                .map_err(|source| CoreBootstrapError::Auth(AuthBootstrapError::Random(source)))?,
        )
    } else {
        None
    };

    Ok(CoreBootstrap {
        local_service: Some(LocalServiceBootstrap {
            storage,
            auth,
            external_catalog,
            local_session,
        }),
    })
}

#[cfg(test)]
#[path = "tests/bootstrap.rs"]
mod tests;
