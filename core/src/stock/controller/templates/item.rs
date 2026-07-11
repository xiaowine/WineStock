//! 物品属性模板 HTTP DTO 和 handler。
//!
//! 本模块属于 stock HTTP 层，模板只提供可选预设，不能禁止物品添加自定义属性。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{TemplateCopyRequest, TemplateFieldDef, TemplateFieldResponse};
use crate::{
    http::{ValidatedJson, ValidatedPath},
    security::CurrentUser,
    state::CoreState,
    stock::service::{self, StockApiError},
    validation::{validate_not_blank, validate_optional_not_blank},
};

/// 创建物品属性模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemAttributeTemplateCreateRequest {
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 推荐的默认入库模板 ID。
    #[garde(skip)]
    pub default_inbound_template_id: Option<i64>,
    /// 物品属性预设字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldDef>,
}

/// 更新物品属性模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemAttributeTemplateUpdateRequest {
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 推荐的默认入库模板 ID。
    #[garde(skip)]
    pub default_inbound_template_id: Option<i64>,
    /// 字段存在时整体替换。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldDef>>,
}

/// 物品属性模板响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct ItemAttributeTemplateResponse {
    /// 模板 ID。
    #[garde(skip)]
    pub id: i64,
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 推荐的默认入库模板 ID。
    #[garde(skip)]
    pub default_inbound_template_id: Option<i64>,
    /// 物品属性预设字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldResponse>,
    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,
    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,
}

/// 创建物品属性模板。
#[utoipa::path(post, path = "/api/item-attribute-templates", tag = "item-attribute-templates", request_body = ItemAttributeTemplateCreateRequest, security(("bearerAuth" = [])), responses((status = 201, body = ItemAttributeTemplateResponse)))]
pub(crate) async fn create_item_attribute_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<ItemAttributeTemplateCreateRequest>,
) -> Result<(StatusCode, Json<ItemAttributeTemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item_attribute_template(&state, &user, request).await?),
    ))
}
/// 查询物品属性模板列表。
#[utoipa::path(get, path = "/api/item-attribute-templates", tag = "item-attribute-templates", security(("bearerAuth" = [])), responses((status = 200, body = Vec<ItemAttributeTemplateResponse>)))]
pub(crate) async fn list_item_attribute_templates(
    State(state): State<CoreState>,
) -> Result<Json<Vec<ItemAttributeTemplateResponse>>, StockApiError> {
    Ok(Json(service::list_item_attribute_templates(&state).await?))
}
/// 查询物品属性模板详情。
#[utoipa::path(get, path = "/api/item-attribute-templates/{id}", tag = "item-attribute-templates", params(("id" = i64, Path)), security(("bearerAuth" = [])), responses((status = 200, body = ItemAttributeTemplateResponse)))]
pub(crate) async fn get_item_attribute_template(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<ItemAttributeTemplateResponse>, StockApiError> {
    Ok(Json(
        service::get_item_attribute_template(&state, id).await?,
    ))
}
/// 更新物品属性模板。
#[utoipa::path(put, path = "/api/item-attribute-templates/{id}", tag = "item-attribute-templates", params(("id" = i64, Path)), request_body = ItemAttributeTemplateUpdateRequest, security(("bearerAuth" = [])), responses((status = 200, body = ItemAttributeTemplateResponse)))]
pub(crate) async fn update_item_attribute_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<ItemAttributeTemplateUpdateRequest>,
) -> Result<Json<ItemAttributeTemplateResponse>, StockApiError> {
    Ok(Json(
        service::update_item_attribute_template(&state, &user, id, request).await?,
    ))
}
/// 软删除未被有效物品引用的属性模板。
#[utoipa::path(delete, path = "/api/item-attribute-templates/{id}", tag = "item-attribute-templates", params(("id" = i64, Path)), security(("bearerAuth" = [])), responses((status = 204)))]
pub(crate) async fn delete_item_attribute_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_item_attribute_template(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
/// 复制物品属性模板。
#[utoipa::path(post, path = "/api/item-attribute-templates/{id}/copy", tag = "item-attribute-templates", params(("id" = i64, Path)), request_body = TemplateCopyRequest, security(("bearerAuth" = [])), responses((status = 201, body = ItemAttributeTemplateResponse)))]
pub(crate) async fn copy_item_attribute_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<TemplateCopyRequest>,
) -> Result<(StatusCode, Json<ItemAttributeTemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::copy_item_attribute_template(&state, &user, id, request).await?),
    ))
}
