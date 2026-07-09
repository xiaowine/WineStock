//! 替代料 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责替代料整体替换、查询和删除入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
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
};

use crate::stock::service::{self, StockApiError};
/// 替代料替换条目。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct SubstituteReplacementItem {
    /// 替代料物品 ID。
    #[garde(range(min = 1))]
    pub substitute_item_id: i64,

    /// 替代优先级，数值越小越优先。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
}

/// 替代料整体替换请求；提交列表会整体替换当前物品的替代料关系。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubstituteReplaceRequest {
    /// 替代料列表；允许空列表，用于清空当前物品的所有替代料关系。
    #[garde(dive)]
    pub substitutes: Vec<SubstituteReplacementItem>,
}

/// 指定物品的替代料响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemSubstituteResponse {
    /// 主物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 替代料物品 ID。
    #[garde(skip)]
    pub substitute_item_id: i64,

    /// 替代料物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub substitute_item_name: String,

    /// 替代料当前库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 替代优先级。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 全量替代料关系响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct SubstituteRelationResponse {
    /// 主物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 主物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub item_name: String,

    /// 主物品 SKU。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub item_sku: String,

    /// 替代料物品 ID。
    #[garde(skip)]
    pub substitute_item_id: i64,

    /// 替代料物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub substitute_item_name: String,

    /// 替代料物品 SKU。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub substitute_item_sku: String,

    /// 替代料当前库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 替代优先级。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

#[utoipa::path(
    put,
    path = "/api/substitutes/{item_id}",
    tag = "substitutes",
    request_body = SubstituteReplaceRequest,
    params(("item_id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitutes replaced", body = Vec<ItemSubstituteResponse>),
        (status = 400, description = "Invalid substitute request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Substitute manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 整体替换指定物品的替代料列表。
pub(crate) async fn replace_substitutes(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(item_id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<SubstituteReplaceRequest>,
) -> Result<Json<Vec<ItemSubstituteResponse>>, StockApiError> {
    Ok(Json(
        service::replace_substitutes(&state, &current_user, item_id, request).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/substitutes",
    tag = "substitutes",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "All substitute relations", body = Vec<SubstituteRelationResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Substitute read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询全部物品替代料关系。
pub(crate) async fn list_substitute_relations(
    State(state): State<CoreState>,
) -> Result<Json<Vec<SubstituteRelationResponse>>, StockApiError> {
    Ok(Json(service::list_substitute_relations(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/substitutes/{item_id}",
    tag = "substitutes",
    params(("item_id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitute list", body = Vec<ItemSubstituteResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Substitute read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询指定物品的替代料列表。
pub(crate) async fn list_item_substitutes(
    State(state): State<CoreState>,
    ValidatedPath(item_id): ValidatedPath<i64>,
) -> Result<Json<Vec<ItemSubstituteResponse>>, StockApiError> {
    Ok(Json(service::list_item_substitutes(&state, item_id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/substitutes/{item_id}/{substitute_item_id}",
    tag = "substitutes",
    params(
        ("item_id" = i64, Path, description = "Item ID"),
        ("substitute_item_id" = i64, Path, description = "Substitute item ID")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Substitute relation deleted"),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Substitute manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item or substitute relation not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 删除单个替代料关系。
pub(crate) async fn delete_substitute_relation(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath((item_id, substitute_item_id)): ValidatedPath<(i64, i64)>,
) -> Result<StatusCode, StockApiError> {
    service::delete_substitute_relation(&state, &current_user, item_id, substitute_item_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
