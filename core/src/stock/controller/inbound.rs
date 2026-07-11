//! 入库单 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责入库单创建、查询、筛选值、审批和拒绝入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    http::{ValidatedJson, ValidatedPath, ValidatedQuery},
    security::CurrentUser,
    state::CoreState,
};

use crate::stock::service::{self, PaginatedResponse, StockApiError};

use super::common::{validate_positive_number, OrderStatus};

/// 创建入库单时采用的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InboundSubmissionMode {
    /// 创建待审批单据，后续由拥有审核权限的用户处理。
    PendingApproval,

    /// 创建并直接完成入库，仅同时拥有创建和审核权限的用户可用。
    Direct,
}

/// 创建入库单明细请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundItemRequest {
    /// 入库物品 ID，必须指向未软删除物品。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 入库数量，必须大于 0。
    #[garde(custom(validate_positive_number))]
    pub quantity: f64,

    /// 入库单价，不允许为负。
    #[garde(skip)]
    pub unit_price: f64,

    /// 入库库位 ID，必须指向未软删除库位。
    #[garde(range(min = 1))]
    pub location_id: i64,

    /// 外部批次号；为空时审批阶段生成内部批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期文本，首版按调用方输入保存。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 可选入库模板 ID；为空时可由物品属性模板推荐值推导。
    #[garde(skip)]
    pub inbound_template_id: Option<i64>,

    /// 模板扩展属性；创建和审批阶段都按物品当前模板校验，file 值使用 `{ "file_id": id }`。
    #[garde(skip)]
    pub ext_attributes: Option<Value>,
}

/// 创建入库单请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundCreateRequest {
    /// 本次提交是进入待审批状态还是直接完成入库。
    #[garde(skip)]
    pub submission_mode: InboundSubmissionMode,

    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 入库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 入库明细，至少一条。
    #[garde(dive)]
    pub items: Vec<InboundItemRequest>,
}

/// 入库单分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct InboundListQuery {
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

    /// 按入库单、明细、关联物品和模板值模糊搜索。
    pub search: Option<String>,
}

/// 入库单明细响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct InboundItemResponse {
    /// 明细 ID。
    #[garde(skip)]
    pub id: i64,

    /// 所属入库单 ID。
    #[garde(skip)]
    pub order_id: i64,

    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 入库数量。
    #[garde(skip)]
    pub quantity: f64,

    /// 入库单价。
    #[garde(skip)]
    pub unit_price: f64,

    /// 入库库位 ID。
    #[garde(skip)]
    pub location_id: i64,

    /// 入库库位编码。
    #[garde(skip)]
    pub location_code: String,

    /// 入库库位名称。
    #[garde(skip)]
    pub location_name: String,

    /// 批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期文本。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 本明细使用的入库模板 ID。
    #[garde(skip)]
    pub inbound_template_id: Option<i64>,

    /// 模板扩展属性。
    #[garde(skip)]
    pub ext_attributes: Option<Value>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 入库单响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct InboundResponse {
    /// 入库单 ID。
    #[garde(skip)]
    pub id: i64,

    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 入库状态。
    #[garde(skip)]
    pub status: OrderStatus,

    /// 仅创建接口返回本次采用的提交方式；查询和后续审批响应不重复推断历史创建意图。
    #[garde(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_mode: Option<InboundSubmissionMode>,

    /// 入库备注。
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

    /// 入库明细。
    #[garde(dive)]
    pub items: Vec<InboundItemResponse>,
}

#[utoipa::path(
    post,
    path = "/api/inbound",
    tag = "inbound",
    request_body = InboundCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Inbound order created", body = InboundResponse),
        (status = 400, description = "Invalid inbound request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound create permission required, and direct mode also requires inbound approve permission", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item, location, or template not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Image reference unavailable", body = crate::http::ApiErrorResponse)
    )
)]
/// 按请求模式创建待审批单据或在同一事务内直接完成入库。
pub(crate) async fn create_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<InboundCreateRequest>,
) -> Result<(StatusCode, Json<InboundResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_inbound(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/inbound",
    tag = "inbound",
    params(InboundListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order list", body = PaginatedResponse<InboundResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询入库单。
pub(crate) async fn list_inbound(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<InboundListQuery>,
) -> Result<Json<PaginatedResponse<InboundResponse>>, StockApiError> {
    Ok(Json(service::list_inbound(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/inbound/filter-values",
    tag = "inbound",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound history filter values", body = super::FilterValuesResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询入库历史视角下的筛选值。
pub(crate) async fn inbound_filter_values(
    State(state): State<CoreState>,
) -> Result<Json<super::FilterValuesResponse>, StockApiError> {
    Ok(Json(service::inbound_filter_values(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/inbound/{id}",
    tag = "inbound",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order detail", body = InboundResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Inbound order not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询入库单详情。
pub(crate) async fn get_inbound(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(service::get_inbound(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/stock-approvals/inbound/{id}/approve",
    tag = "stock-approvals",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order approved", body = InboundResponse),
        (status = 400, description = "Invalid inbound attributes", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound approve permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Inbound order not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Inbound order is not pending", body = crate::http::ApiErrorResponse)
    )
)]
/// 审批 pending 入库单；审批事务会写批次、库存流水和审计事件。
pub(crate) async fn approve_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(
        service::approve_inbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/stock-approvals/inbound/{id}/reject",
    tag = "stock-approvals",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order rejected", body = InboundResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Inbound approve permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Inbound order not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Inbound order is not pending", body = crate::http::ApiErrorResponse)
    )
)]
/// 拒绝 pending 入库单；拒绝不改变库存。
pub(crate) async fn reject_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(
        service::reject_inbound(&state, &current_user, id).await?,
    ))
}
