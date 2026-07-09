//! 库存事件日志 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责审计事件列表查询入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::state::CoreState;

use crate::stock::service::{self, PaginatedResponse, StockApiError};
/// 事件日志分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct EventListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按实体类型筛选。
    pub entity_type: Option<String>,

    /// 按实体 ID 筛选。
    pub entity_id: Option<i64>,

    /// 按操作动作筛选。
    pub action: Option<String>,

    /// 按操作人用户 ID 筛选。
    pub user_id: Option<i64>,

    /// 操作时间起点，使用 SQLite UTC 字符串格式。
    pub date_from: Option<String>,

    /// 操作时间终点，使用 SQLite UTC 字符串格式。
    pub date_to: Option<String>,
}

/// 事件日志响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct EventLogResponse {
    /// 事件 ID。
    #[garde(skip)]
    pub id: i64,

    /// 操作时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub timestamp: String,

    /// 操作人用户 ID。
    #[garde(skip)]
    pub user_id: Option<i64>,

    /// 操作人用户名；用户被删除或外键置空时为空。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub username: Option<String>,

    /// 被操作实体类型。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub entity_type: String,

    /// 被操作实体 ID。
    #[garde(skip)]
    pub entity_id: Option<i64>,

    /// 操作动作。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub action: String,

    /// 事件详情 JSON。
    #[garde(skip)]
    pub details: Value,
}

#[utoipa::path(
    get,
    path = "/api/events",
    tag = "events",
    params(EventListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Event log list", body = PaginatedResponse<EventLogResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Audit read permission required", body = String)
    )
)]
/// 分页查询事件日志。
pub(crate) async fn list_events(
    State(state): State<CoreState>,
    Query(query): Query<EventListQuery>,
) -> Result<Json<PaginatedResponse<EventLogResponse>>, StockApiError> {
    Ok(Json(service::list_events(&state, query).await?))
}
