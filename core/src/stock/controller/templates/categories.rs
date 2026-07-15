//! 物品分类 HTTP DTO 和 handler。
//!
//! 本模块属于 stock HTTP 层，分类只表达归类关系，不携带模板字段。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    http::{ValidatedJson, ValidatedPath},
    security::CurrentUser,
    state::CoreState,
    stock::service::{self, StockApiError},
    validation::{validate_not_blank, validate_optional_not_blank},
};

/// 创建物品分类请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemCategoryCreateRequest {
    /// 分类名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 分类说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: Option<i32>,
}

/// 更新物品分类请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemCategoryUpdateRequest {
    /// 分类名称。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 分类说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: Option<i32>,
}

/// 物品分类响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct ItemCategoryResponse {
    /// 分类 ID。
    #[garde(skip)]
    pub id: i64,
    /// 分类名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 分类说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
    /// 当前有效物品直接使用该分类的数量。
    #[garde(skip)]
    pub item_usage_count: u64,
    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,
    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,
}

/// 删除分类后受影响的当前有效物品数量。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemCategoryDeleteResponse {
    /// 删除时仍直接引用该分类的有效物品数量。
    pub affected_active_item_count: u64,
}

/// 创建物品分类。
#[utoipa::path(post, path = "/api/item-categories", tag = "item-categories", request_body = ItemCategoryCreateRequest, security(("bearerAuth" = [])), responses((status = 201, body = ItemCategoryResponse)))]
pub(crate) async fn create_item_category(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<ItemCategoryCreateRequest>,
) -> Result<(StatusCode, Json<ItemCategoryResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item_category(&state, &user, request).await?),
    ))
}
/// 查询物品分类列表。
#[utoipa::path(get, path = "/api/item-categories", tag = "item-categories", security(("bearerAuth" = [])), responses((status = 200, body = Vec<ItemCategoryResponse>)))]
pub(crate) async fn list_item_categories(
    State(state): State<CoreState>,
) -> Result<Json<Vec<ItemCategoryResponse>>, StockApiError> {
    Ok(Json(service::list_item_categories(&state).await?))
}
/// 查询物品分类详情。
#[utoipa::path(get, path = "/api/item-categories/{id}", tag = "item-categories", params(("id" = i64, Path)), security(("bearerAuth" = [])), responses((status = 200, body = ItemCategoryResponse)))]
pub(crate) async fn get_item_category(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<ItemCategoryResponse>, StockApiError> {
    Ok(Json(service::get_item_category(&state, id).await?))
}
/// 更新物品分类。
#[utoipa::path(put, path = "/api/item-categories/{id}", tag = "item-categories", params(("id" = i64, Path)), request_body = ItemCategoryUpdateRequest, security(("bearerAuth" = [])), responses((status = 200, body = ItemCategoryResponse)))]
pub(crate) async fn update_item_category(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<ItemCategoryUpdateRequest>,
) -> Result<Json<ItemCategoryResponse>, StockApiError> {
    Ok(Json(
        service::update_item_category(&state, &user, id, request).await?,
    ))
}
/// 软删除物品分类。
#[utoipa::path(delete, path = "/api/item-categories/{id}", tag = "item-categories", params(("id" = i64, Path)), security(("bearerAuth" = [])), responses((status = 200, body = ItemCategoryDeleteResponse)))]
pub(crate) async fn delete_item_category(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<ItemCategoryDeleteResponse>, StockApiError> {
    Ok(Json(
        service::delete_item_category(&state, &user, id).await?,
    ))
}
