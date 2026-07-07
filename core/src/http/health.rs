//! core 全局健康检查接口。
//!
//! 本模块属于 `core axum library` 层，负责最小健康检查响应。
//! 它不依赖具体业务领域，也不暴露平台私有状态。

use axum::Json;
use serde::{Deserialize, Serialize};

/// 健康检查接口返回体。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// 服务状态，当前健康状态返回 `ok`。
    pub status: String,

    /// 返回当前响应的服务标识。
    pub service: String,
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
pub(crate) async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        service: "winestock-core".to_owned(),
    })
}
