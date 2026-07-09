//! 库位分组、库位和移库 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责库位树、库位主数据和整批次移库入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, security::CurrentUser, state::CoreState};

use crate::stock::service::{self, StockApiError};

/// 创建库位分组请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocationGroupCreateRequest {
    /// 上级分组 ID；为空表示根分组。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值；为空时按 0 保存。
    #[garde(skip)]
    pub sort_order: Option<i32>,
}

/// 更新库位分组请求；`parent_id` 为空表示移动到根分组。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocationGroupUpdateRequest {
    /// 上级分组 ID；为空表示根分组。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值；为空时按 0 保存。
    #[garde(skip)]
    pub sort_order: Option<i32>,
}

/// 库位分组响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct LocationGroupResponse {
    /// 分组 ID。
    #[garde(skip)]
    pub id: i64,

    /// 上级分组 ID。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值。
    #[garde(skip)]
    pub sort_order: i32,

    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,
}

/// 库位分组树节点响应，包含直接子分组和直接库位。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, garde::Validate)]
pub(crate) struct LocationGroupTreeNode {
    /// 分组 ID。
    #[garde(skip)]
    pub id: i64,

    /// 上级分组 ID。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值。
    #[garde(skip)]
    pub sort_order: i32,

    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,

    /// 当前分组的直接库位。
    #[garde(dive)]
    pub locations: Vec<LocationResponse>,

    /// 当前分组的直接子分组。
    #[garde(dive)]
    pub children: Vec<LocationGroupTreeNode>,
}

/// 库位列表查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct LocationListQuery {
    /// 按所属分组 ID 筛选。
    pub group_id: Option<i64>,

    /// 按库位编码或名称模糊搜索。
    pub search: Option<String>,
}

/// 创建库位请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocationCreateRequest {
    /// 所属分组 ID。
    #[garde(range(min = 1))]
    pub group_id: i64,

    /// 库位编码，未删除库位内全局唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub code: String,

    /// 库位名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值；为空时按 0 保存。
    #[garde(skip)]
    pub sort_order: Option<i32>,
}

/// 更新库位请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocationUpdateRequest {
    /// 所属分组 ID。
    #[garde(range(min = 1))]
    pub group_id: i64,

    /// 库位编码，未删除库位内全局唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub code: String,

    /// 库位名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值；为空时按 0 保存。
    #[garde(skip)]
    pub sort_order: Option<i32>,
}

/// 库位响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct LocationResponse {
    /// 库位 ID。
    #[garde(skip)]
    pub id: i64,

    /// 所属分组 ID。
    #[garde(skip)]
    pub group_id: i64,

    /// 所属分组名称。
    #[garde(skip)]
    pub group_name: String,

    /// 库位编码。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub code: String,

    /// 库位名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 排序值。
    #[garde(skip)]
    pub sort_order: i32,

    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,
}

/// 整批次移库请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocationTransferCreateRequest {
    /// 需要移动的库存批次 ID。
    #[garde(range(min = 1))]
    pub batch_id: i64,

    /// 调用方确认的当前原库位 ID。
    #[garde(range(min = 1))]
    pub from_location_id: i64,

    /// 目标库位 ID。
    #[garde(range(min = 1))]
    pub to_location_id: i64,

    /// 移库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
}

/// 整批次移库响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct LocationTransferResponse {
    /// 移库记录 ID。
    #[garde(skip)]
    pub id: i64,

    /// 被移动批次 ID。
    #[garde(skip)]
    pub batch_id: i64,

    /// 被移动物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 原库位 ID。
    #[garde(skip)]
    pub from_location_id: i64,

    /// 目标库位 ID。
    #[garde(skip)]
    pub to_location_id: i64,

    /// 本次移动的当前批次余额。
    #[garde(skip)]
    pub quantity: f64,

    /// 移库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 操作人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 移库时间。
    #[garde(skip)]
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/location-groups/tree",
    tag = "locations",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Location group tree", body = serde_json::Value),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location read permission required", body = String)
    )
)]
/// 查询库位分组树。
pub(crate) async fn list_location_group_tree(
    State(state): State<CoreState>,
) -> Result<Json<Vec<LocationGroupTreeNode>>, StockApiError> {
    Ok(Json(service::list_location_group_tree(&state).await?))
}

#[utoipa::path(
    post,
    path = "/api/location-groups",
    tag = "locations",
    request_body = LocationGroupCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Location group created", body = LocationGroupResponse),
        (status = 400, description = "Invalid location group request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Parent location group not found", body = String),
        (status = 409, description = "Location group name already exists", body = String)
    )
)]
/// 创建库位分组。
pub(crate) async fn create_location_group(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<LocationGroupCreateRequest>,
) -> Result<(StatusCode, Json<LocationGroupResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_location_group(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    put,
    path = "/api/location-groups/{id}",
    tag = "locations",
    request_body = LocationGroupUpdateRequest,
    params(("id" = i64, Path, description = "Location group ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Location group updated", body = LocationGroupResponse),
        (status = 400, description = "Invalid location group request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Location group not found", body = String),
        (status = 409, description = "Location group name already exists", body = String)
    )
)]
/// 更新库位分组。
pub(crate) async fn update_location_group(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<LocationGroupUpdateRequest>,
) -> Result<Json<LocationGroupResponse>, StockApiError> {
    Ok(Json(
        service::update_location_group(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/location-groups/{id}",
    tag = "locations",
    params(("id" = i64, Path, description = "Location group ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Location group deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Location group not found", body = String),
        (status = 409, description = "Location group is in use", body = String)
    )
)]
/// 删除库位分组。
pub(crate) async fn delete_location_group(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_location_group(&state, &current_user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/locations",
    tag = "locations",
    params(LocationListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Location list", body = Vec<LocationResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location read permission required", body = String)
    )
)]
/// 查询库位列表。
pub(crate) async fn list_locations(
    State(state): State<CoreState>,
    Query(query): Query<LocationListQuery>,
) -> Result<Json<Vec<LocationResponse>>, StockApiError> {
    Ok(Json(service::list_locations(&state, query).await?))
}

#[utoipa::path(
    post,
    path = "/api/locations",
    tag = "locations",
    request_body = LocationCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Location created", body = LocationResponse),
        (status = 400, description = "Invalid location request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Location group not found", body = String),
        (status = 409, description = "Location code already exists", body = String)
    )
)]
/// 创建库位。
pub(crate) async fn create_location(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<LocationCreateRequest>,
) -> Result<(StatusCode, Json<LocationResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_location(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    put,
    path = "/api/locations/{id}",
    tag = "locations",
    request_body = LocationUpdateRequest,
    params(("id" = i64, Path, description = "Location ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Location updated", body = LocationResponse),
        (status = 400, description = "Invalid location request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Location not found", body = String),
        (status = 409, description = "Location code already exists", body = String)
    )
)]
/// 更新库位。
pub(crate) async fn update_location(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<LocationUpdateRequest>,
) -> Result<Json<LocationResponse>, StockApiError> {
    Ok(Json(
        service::update_location(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/locations/{id}",
    tag = "locations",
    params(("id" = i64, Path, description = "Location ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Location deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Location not found", body = String),
        (status = 409, description = "Location is in use", body = String)
    )
)]
/// 删除库位。
pub(crate) async fn delete_location(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_location(&state, &current_user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/location-transfers",
    tag = "locations",
    request_body = LocationTransferCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Location transfer created", body = LocationTransferResponse),
        (status = 400, description = "Invalid location transfer request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Location manage permission required", body = String),
        (status = 404, description = "Batch or location not found", body = String)
    )
)]
/// 创建整批次移库记录并移动批次库位。
pub(crate) async fn create_location_transfer(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<LocationTransferCreateRequest>,
) -> Result<(StatusCode, Json<LocationTransferResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_location_transfer(&state, &current_user, request).await?),
    ))
}
