//! 库存物品 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责物品 CRUD、列表筛选值的请求、响应和 Axum 入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, security::CurrentUser, state::CoreState};

use crate::stock::service::{self, PaginatedResponse, StockApiError};
/// 创建库存物品请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemCreateRequest {
    /// 物品名称，服务端会裁剪首尾空白。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU，未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,
}

/// 更新库存物品请求；字段为空表示不修改。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemUpdateRequest {
    /// 物品名称，存在时服务端会裁剪首尾空白。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 物品 SKU，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub sku: Option<String>,

    /// 关联模板 ID；当前首版接口不通过 null 清空该字段。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,

    /// 物品描述；当前首版接口不通过 null 清空该字段。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,
}

/// 库存物品分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按物品基础字段、模板元数据和当前库存模板值模糊搜索。
    pub search: Option<String>,

    /// 按关联模板 ID 筛选。
    pub category_id: Option<i64>,
}

/// 库存物品响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemResponse {
    /// 物品 ID。
    #[garde(skip)]
    pub id: i64,

    /// 物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,
}

/// 库存物品详情响应，包含基础资料和当前库存快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemDetailResponse {
    /// 物品 ID。
    #[garde(skip)]
    pub id: i64,

    /// 物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,

    /// 当前剩余库存总量，只统计仍有余额的批次。
    #[garde(skip)]
    pub current_quantity: f64,

    /// 当前库存价值，按批次剩余数量乘以批次单价汇总。
    #[garde(skip)]
    pub inventory_value: f64,

    /// 当前库存按库位聚合后的分布。
    #[garde(skip)]
    pub locations: Vec<ItemLocationStockResponse>,

    /// 当前仍有余额的批次摘要。
    #[garde(skip)]
    pub batches: Vec<ItemBatchStockResponse>,
}

/// 物品详情中的库位库存分布。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemLocationStockResponse {
    /// 库位 ID。
    #[garde(skip)]
    pub location_id: i64,

    /// 库位编码。
    #[garde(skip)]
    pub location_code: String,

    /// 库位名称。
    #[garde(skip)]
    pub location_name: String,

    /// 该库位当前剩余库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 该库位当前库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 该库位当前仍有余额的批次数。
    #[garde(skip)]
    pub batch_count: i64,
}

/// 物品详情中的当前批次摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemBatchStockResponse {
    /// 批次 ID。
    #[garde(skip)]
    pub id: i64,

    /// 批次号。
    #[garde(skip)]
    pub batch_no: String,

    /// 批次库位 ID。
    #[garde(skip)]
    pub location_id: i64,

    /// 批次库位编码。
    #[garde(skip)]
    pub location_code: String,

    /// 批次库位名称。
    #[garde(skip)]
    pub location_name: String,

    /// 入库时的初始数量。
    #[garde(skip)]
    pub initial_quantity: f64,

    /// 当前剩余数量。
    #[garde(skip)]
    pub remaining_quantity: f64,

    /// 批次单价。
    #[garde(skip)]
    pub unit_cost: f64,

    /// 当前批次库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 入库审批时间。
    #[garde(skip)]
    pub received_at: String,

    /// 有效期。
    #[garde(skip)]
    pub expires_at: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/items",
    tag = "stock",
    request_body = ItemCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Item created", body = ItemResponse),
        (status = 400, description = "Invalid item request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 409, description = "SKU already exists", body = String)
    )
)]
/// 创建库存物品。
pub(crate) async fn create_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<ItemCreateRequest>,
) -> Result<(StatusCode, Json<ItemResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/items",
    tag = "stock",
    params(ItemListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item list", body = PaginatedResponse<ItemResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item read permission required", body = String)
    )
)]
/// 分页查询库存物品。
pub(crate) async fn list_items(
    State(state): State<CoreState>,
    Query(query): Query<ItemListQuery>,
) -> Result<Json<PaginatedResponse<ItemResponse>>, StockApiError> {
    Ok(Json(service::list_items(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/filter-values",
    tag = "stock",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Current inventory item filter values", body = super::FilterValuesResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item read permission required", body = String)
    )
)]
/// 查询当前库存视角下的物品筛选值。
pub(crate) async fn item_filter_values(
    State(state): State<CoreState>,
) -> Result<Json<super::FilterValuesResponse>, StockApiError> {
    Ok(Json(service::item_filter_values(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item detail", body = ItemDetailResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item read permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 查询单个库存物品。
pub(crate) async fn get_item(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<ItemDetailResponse>, StockApiError> {
    Ok(Json(service::get_item(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/items/{id}",
    tag = "stock",
    request_body = ItemUpdateRequest,
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item updated", body = ItemResponse),
        (status = 400, description = "Invalid item request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 404, description = "Item not found", body = String),
        (status = 409, description = "SKU already exists", body = String)
    )
)]
/// 更新库存物品。
pub(crate) async fn update_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<ItemUpdateRequest>,
) -> Result<Json<ItemResponse>, StockApiError> {
    Ok(Json(
        service::update_item(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/items/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Item deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 软删除库存物品。
pub(crate) async fn delete_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_item(&state, &current_user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
