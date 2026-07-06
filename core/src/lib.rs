#![forbid(unsafe_code)]

//! WineStock 共享 Rust/Axum 服务核心。
//!
//! 本 crate 属于 `core axum library` 层，拥有 API 路由、OpenAPI 文档、网络绑定、
//! 本地服务启动依赖、鉴权初始化和持久化集成。
//! 它不拥有 server 进程生命周期、桌面/Android shell、WebView 或前端打包产物。

mod auth;
mod bootstrap;
mod persistence;
mod rbac;
mod server;

use axum::{routing::get, routing::post, Json, Router};
use serde::Serialize;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;
use winestock_shared::{
    AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};

pub use auth::{
    AuthApiError, AuthBootstrap, AuthSettings, AuthSigningKey, CurrentUser, SigningKeyStatus,
};
pub use bootstrap::{
    bootstrap_from_config, CoreBootstrap, CoreBootstrapError, LocalServiceBootstrap,
};
pub use persistence::{StorageBootstrapError, StorageRuntime};
pub use rbac::RbacBootstrapError;
pub use server::{bind_server, BoundServer, ServerStartError};
pub use winestock_shared as shared;

/// OpenAPI JSON 文档的服务路径。
pub const OPENAPI_JSON_PATH: &str = "/api-docs/openapi.json";

/// Swagger UI 的服务路径。
pub const SWAGGER_UI_PATH: &str = "/swagger-ui";

// 这里集中声明接口文档元信息，具体路径由带 #[utoipa::path] 的处理函数收集。
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        health_check,
        auth::register,
        auth::login,
        auth::refresh,
        auth::logout,
        auth::me
    ),
    components(schemas(
        HealthResponse,
        AuthRegisterRequest,
        AuthLoginRequest,
        AuthRefreshRequest,
        AuthLogoutRequest,
        AuthUserResponse,
        AuthTokenResponse
    )),
    modifiers(&SecurityAddon),
    info(title = "WineStock API", version = "0.1.0"),
    tags(
        (name = "system", description = "Service status endpoints"),
        (name = "auth", description = "Authentication endpoints")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 健康检查接口返回体。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// 服务状态，当前健康状态返回 `ok`。
    pub status: String,

    /// 返回当前响应的服务标识。
    pub service: String,
}

/// 构建平台壳共用的 Axum 路由器。
pub fn build_router() -> Router {
    // API 文档工具只服务接口说明，不承载桌面或 Android 的平台前端资源。
    Router::new()
        .route("/api/health", get(health_check))
        .merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, ApiDoc::openapi()))
}

/// 构建已接入本地存储和鉴权状态的 Axum 路由器。
pub fn build_router_with_local_service(local_service: &LocalServiceBootstrap) -> Router {
    let auth_state = auth::AuthRuntime::from_local_service(local_service);

    // 鉴权路由依赖数据库和签名密钥状态，只有本地服务模式才能挂载。
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .with_state(auth_state)
        .merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, ApiDoc::openapi()))
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
/// 返回平台壳和外部监控可调用的最小健康检查响应。
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        service: "winestock-core".to_owned(),
    })
}
