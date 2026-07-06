#![forbid(unsafe_code)]

//! WineStock 共享 Rust/Axum 服务核心。

mod auth;
mod bootstrap;
mod persistence;
mod server;

use axum::{Json, Router};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

pub use auth::{AuthBootstrap, AuthSettings, AuthSigningKey, SigningKeyStatus};
pub use bootstrap::{
    bootstrap_from_config, CoreBootstrap, CoreBootstrapError, LocalServiceBootstrap,
};
pub use persistence::{StorageBootstrapError, StorageRuntime};
pub use server::{bind_server, BoundServer, ServerStartError};
pub use winestock_shared as shared;

/// OpenAPI JSON 文档的服务路径。
pub const OPENAPI_JSON_PATH: &str = "/api-docs/openapi.json";

/// Swagger UI 的服务路径。
pub const SWAGGER_UI_PATH: &str = "/swagger-ui";

// 这里集中声明接口文档元信息，具体路径由带 #[utoipa::path] 的处理函数收集。
#[derive(utoipa::OpenApi)]
#[openapi(
    info(title = "WineStock API", version = "0.1.0"),
    tags(
        (name = "system", description = "Service status endpoints")
    )
)]
struct ApiDoc;

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
    let (api_router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health_check))
        .split_for_parts();

    // Swagger UI 只服务 API 文档，不承载桌面或 Android 的平台前端资源。
    Router::new()
        .merge(api_router)
        .merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, openapi))
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "system",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        service: "winestock-core".to_owned(),
    })
}
