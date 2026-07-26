//! core 全局 Axum state 根对象。
//!
//! 本模块属于 `core axum library` 层，负责把本地存储运行时和安全前置层运行时
//! 组合成统一的请求共享状态。它不负责 HTTP 路由装配或平台启动流程。

use sea_orm::DatabaseConnection;

use crate::{
    bootstrap::LocalServiceBootstrap, external::ExternalCatalogRuntime,
    persistence::StorageRuntime, security::SecurityRuntime,
};

/// Axum 路由共享的全局状态根对象。
#[derive(Debug, Clone)]
pub(crate) struct CoreState {
    storage: StorageRuntime,
    security: SecurityRuntime,
    external_catalog: ExternalCatalogRuntime,
}

impl CoreState {
    /// 从已经完成 bootstrap 的本地服务状态构造统一 `CoreState`。
    pub(crate) fn from_local_service(local_service: &LocalServiceBootstrap) -> Self {
        Self {
            storage: local_service.storage.clone(),
            security: SecurityRuntime::from_auth_bootstrap(&local_service.auth),
            external_catalog: local_service.external_catalog.clone(),
        }
    }

    /// 返回全局共享的 SQLite 连接，供各领域 repository 复用。
    pub(crate) fn database(&self) -> &DatabaseConnection {
        &self.storage.database
    }

    /// 返回安全前置层运行时。
    pub(crate) fn security(&self) -> &SecurityRuntime {
        &self.security
    }

    /// 返回只读外部商品资料运行时；业务请求不能覆盖其 endpoint。
    pub(crate) fn external_catalog(&self) -> &ExternalCatalogRuntime {
        &self.external_catalog
    }

    /// 返回存储运行时，供后续文件或附件领域读取公共路径信息。
    #[allow(dead_code)]
    pub(crate) fn storage(&self) -> &StorageRuntime {
        &self.storage
    }
}
