#![forbid(unsafe_code)]

//! WineStock 共享 Rust/Axum 服务核心。
//!
//! 本 crate 属于 `core axum library` 层，拥有 API 路由、Debug API 文档、网络绑定、
//! 本地服务启动依赖、security/auth/users/rbac/files/stock 领域能力和持久化集成。
//! 它不拥有 server 进程生命周期、桌面/Android shell、WebView 或前端打包产物。

mod auth;
mod bootstrap;
mod external;
mod files;
mod http;
mod local_service;
mod persistence;
mod rbac;
mod security;
mod server;
mod state;
mod stock;
mod users;
mod validation;

pub use auth::{
    AuthBootstrap, AuthBootstrapError, AuthBootstrapStatus, AuthClientKind, AuthLoginRequest,
    AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest, AuthSettings, AuthSigningKey,
    AuthTokenResponse, AuthUserResponse, SigningKeyStatus,
};
pub use bootstrap::{
    bootstrap_from_config, CoreBootstrap, CoreBootstrapError, LocalServiceBootstrap,
    LocalSessionSecret,
};
pub use external::ExternalCatalogBootstrapError;
pub use files::FileCleanupError;
#[cfg(all(debug_assertions, feature = "swagger-ui"))]
pub use http::SWAGGER_UI_PATH;
pub use http::{build_router, build_router_with_local_service};
#[cfg(debug_assertions)]
pub use http::{openapi_document_json, OPENAPI_JSON_PATH};
pub use local_service::{
    start_local_service, LocalServiceInfo, LocalServiceRuntimeError, RunningLocalService,
};
pub use persistence::{StorageBootstrapError, StorageRuntime};
pub use rbac::RbacBootstrapError;
pub use security::{AuthApiError, CurrentUser};
pub use server::{bind_server, BoundServer, ServerStartError};
pub use stock::StockBootstrapError;
pub use winestock_shared as shared;

#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;
