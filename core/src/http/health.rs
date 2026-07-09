//! core 健康检查 HTTP 入口。
//!
//! 本模块属于 `core axum library` 的全局 HTTP 外壳层，只提供无状态健康检查。
//! 它不访问数据库、不要求鉴权，也不承载业务领域状态。

use axum::Json;
use serde::{Deserialize, Serialize};

/// 健康检查响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct HealthResponse {
    /// 服务状态；当前正常响应固定为 `OK`。
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Health check", body = HealthResponse),
    )
)]
/// 无状态健康检查；用于平台壳或部署侧确认 Axum 已响应。
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
    })
}
