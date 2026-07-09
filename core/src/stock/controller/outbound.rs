//! 出库单 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责出库单创建、查询、搜索、筛选值、审批和拒绝入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    http::{ValidatedJson, ValidatedPath, ValidatedQuery},
    security::CurrentUser,
    state::CoreState,
};

use crate::stock::service::{self, PaginatedResponse, StockApiError};

use super::common::{validate_positive_number, OrderStatus};
/// 创建出库单明细请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutboundItemRequest {
    /// 出库物品 ID，必须指向未软删除物品。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 出库数量，必须大于 0。
    #[garde(custom(validate_positive_number))]
    pub quantity: f64,

    /// 指定扣减批次；为空时审批阶段按 FIFO 扣减。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位 ID；为空时审批阶段按全部当前库存 FIFO 扣减。
    #[garde(skip)]
    pub location_id: Option<i64>,
}

/// 创建出库单请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutboundCreateRequest {
    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 出库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 出库明细，至少一条。
    #[garde(dive)]
    pub items: Vec<OutboundItemRequest>,
}

/// 出库单分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct OutboundListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点，使用 SQLite UTC 字符串格式。
    pub date_from: Option<String>,

    /// 创建时间终点，使用 SQLite UTC 字符串格式。
    pub date_to: Option<String>,

    /// 出库历史自由搜索关键字；为空表示不启用搜索。
    pub search: Option<String>,
}

/// 出库单明细响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct OutboundItemResponse {
    /// 明细 ID。
    #[garde(skip)]
    pub id: i64,

    /// 所属出库单 ID。
    #[garde(skip)]
    pub order_id: i64,

    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 出库数量。
    #[garde(skip)]
    pub quantity: f64,

    /// 指定扣减批次。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位 ID。
    #[garde(skip)]
    pub location_id: Option<i64>,

    /// 出库库位编码。
    #[garde(skip)]
    pub location_code: Option<String>,

    /// 出库库位名称。
    #[garde(skip)]
    pub location_name: Option<String>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 出库单响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct OutboundResponse {
    /// 出库单 ID。
    #[garde(skip)]
    pub id: i64,

    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 出库状态。
    #[garde(skip)]
    pub status: OrderStatus,

    /// 出库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    #[garde(skip)]
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    #[garde(skip)]
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,

    /// 审批时间。
    #[garde(skip)]
    pub approved_at: Option<String>,

    /// 拒绝时间。
    #[garde(skip)]
    pub rejected_at: Option<String>,

    /// 出库明细。
    #[garde(dive)]
    pub items: Vec<OutboundItemResponse>,
}

#[utoipa::path(
    post,
    path = "/api/outbound",
    tag = "outbound",
    request_body = OutboundCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Outbound order created", body = OutboundResponse),
        (status = 400, description = "Invalid outbound request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound create permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 创建 pending 出库单；创建阶段不扣减库存。
pub(crate) async fn create_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<OutboundCreateRequest>,
) -> Result<(StatusCode, Json<OutboundResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_outbound(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/outbound",
    tag = "outbound",
    params(OutboundListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order list", body = PaginatedResponse<OutboundResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询出库单。
pub(crate) async fn list_outbound(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<OutboundListQuery>,
) -> Result<Json<PaginatedResponse<OutboundResponse>>, StockApiError> {
    Ok(Json(service::list_outbound(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/outbound/filter-values",
    tag = "outbound",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound history filter values", body = super::FilterValuesResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询出库历史视角下的筛选值。
pub(crate) async fn outbound_filter_values(
    State(state): State<CoreState>,
) -> Result<Json<super::FilterValuesResponse>, StockApiError> {
    Ok(Json(service::outbound_filter_values(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/outbound/{id}",
    tag = "outbound",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order detail", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Outbound order not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询出库单详情。
pub(crate) async fn get_outbound(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(service::get_outbound(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/stock-approvals/outbound/{id}/approve",
    tag = "stock-approvals",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order approved", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound approve permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Outbound order not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Order is not pending or stock is insufficient", body = crate::http::ApiErrorResponse)
    )
)]
/// 审批 pending 出库单；审批事务会按指定批次或 FIFO 扣减库存。
pub(crate) async fn approve_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(
        service::approve_outbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/stock-approvals/outbound/{id}/reject",
    tag = "stock-approvals",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order rejected", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Outbound approve permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Outbound order not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Outbound order is not pending", body = crate::http::ApiErrorResponse)
    )
)]
/// 拒绝 pending 出库单；拒绝不扣减库存。
pub(crate) async fn reject_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(
        service::reject_outbound(&state, &current_user, id).await?,
    ))
}
