//! 替代料 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责替代料绑定、查询和解绑入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, security::CurrentUser, state::CoreState};

use crate::stock::service::{self, StockApiError};
/// 替代料绑定条目。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct SubstituteItem {
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

/// 替代料绑定请求；提交列表会整体替换当前物品的替代料关系。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubstituteBindRequest {
    /// 替代料列表；允许空列表，用于清空当前物品的所有替代料关系。
    #[garde(dive)]
    pub substitutes: Vec<SubstituteItem>,
}

/// 替代料详情响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct SubstituteDetailResponse {
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

#[utoipa::path(
    post,
    path = "/api/items/{id}/substitutes",
    tag = "stock",
    request_body = SubstituteBindRequest,
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitutes replaced", body = Vec<SubstituteDetailResponse>),
        (status = 400, description = "Invalid substitute request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Substitute manage permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 整体替换指定物品的替代料列表。
pub(crate) async fn bind_substitutes(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<SubstituteBindRequest>,
) -> Result<Json<Vec<SubstituteDetailResponse>>, StockApiError> {
    Ok(Json(
        service::bind_substitutes(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}/substitutes",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitute list", body = Vec<SubstituteDetailResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 查询指定物品的替代料列表。
pub(crate) async fn list_substitutes(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<SubstituteDetailResponse>>, StockApiError> {
    Ok(Json(service::list_substitutes(&state, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/items/{id}/substitutes/{substitute_id}",
    tag = "stock",
    params(
        ("id" = i64, Path, description = "Item ID"),
        ("substitute_id" = i64, Path, description = "Substitute item ID")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Substitute relation deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Substitute manage permission required", body = String),
        (status = 404, description = "Item or substitute relation not found", body = String)
    )
)]
/// 解绑单个替代料关系。
pub(crate) async fn delete_substitute(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((id, substitute_id)): Path<(i64, i64)>,
) -> Result<StatusCode, StockApiError> {
    service::delete_substitute(&state, &current_user, id, substitute_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
