//! 库存物品 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责物品 CRUD、列表筛选值的请求、响应和 Axum 入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, state::CoreState};

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
    ValidatedJson(request): ValidatedJson<ItemCreateRequest>,
) -> Result<(StatusCode, Json<ItemResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item(&state, request).await?),
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
        (status = 403, description = "Stock read permission required", body = String)
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
        (status = 403, description = "Stock read permission required", body = String)
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
        (status = 200, description = "Item detail", body = ItemResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 查询单个库存物品。
pub(crate) async fn get_item(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<ItemResponse>, StockApiError> {
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
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<ItemUpdateRequest>,
) -> Result<Json<ItemResponse>, StockApiError> {
    Ok(Json(service::update_item(&state, id, request).await?))
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
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_item(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
