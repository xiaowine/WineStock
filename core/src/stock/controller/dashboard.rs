//! 库存看板 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责库存总览和趋势查询入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use crate::validation::validate_not_blank;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::stock::service::{self, StockApiError};
use crate::{http::ValidatedQuery, state::CoreState};
/// 呆滞料看板条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct SlowMovingItem {
    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub item_name: String,

    /// 当前库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 当前库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 最近一次出入库流水距今天数。
    #[garde(skip)]
    pub days_since_last_movement: i64,
}

/// 库存看板总览响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct DashboardOverviewResponse {
    /// 未软删除的库存物品种类数。
    #[garde(skip)]
    pub total_items: i64,

    /// 当前库存总数量。
    #[garde(skip)]
    pub total_quantity: f64,

    /// 当前库存总价值。
    #[garde(skip)]
    pub total_value: f64,

    /// 最近三天入库数量。
    #[garde(skip)]
    pub inbound_3d: f64,

    /// 最近三天出库数量。
    #[garde(skip)]
    pub outbound_3d: f64,

    /// 当前呆滞料列表。
    #[garde(dive)]
    pub slow_moving_items: Vec<SlowMovingItem>,
}

/// 看板趋势查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct TrendsQuery {
    /// 趋势天数，默认 30，最大 365；小于 1 时按 1 处理。
    pub days: Option<u64>,
}

/// 每日出入库趋势响应条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct DailyTrend {
    /// 日期，格式为 `YYYY-MM-DD`。
    #[garde(length(equal = 10))]
    pub date: String,

    /// 当日入库数量。
    #[garde(skip)]
    pub inbound_quantity: f64,

    /// 当日出库数量。
    #[garde(skip)]
    pub outbound_quantity: f64,
}

/// 看板趋势响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct TrendsResponse {
    /// 按日期升序排列的趋势数据。
    #[garde(dive)]
    pub daily: Vec<DailyTrend>,
}

#[utoipa::path(
    get,
    path = "/api/dashboard/overview",
    tag = "dashboard",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Dashboard overview", body = DashboardOverviewResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Dashboard read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询库存看板总览。
pub(crate) async fn dashboard_overview(
    State(state): State<CoreState>,
) -> Result<Json<DashboardOverviewResponse>, StockApiError> {
    Ok(Json(service::dashboard_overview(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/trends",
    tag = "dashboard",
    params(TrendsQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Dashboard trends", body = TrendsResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Dashboard read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询库存看板出入库趋势。
pub(crate) async fn dashboard_trends(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<TrendsQuery>,
) -> Result<Json<TrendsResponse>, StockApiError> {
    Ok(Json(service::dashboard_trends(&state, query).await?))
}
