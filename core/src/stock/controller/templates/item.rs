//! 物品属性模板 HTTP DTO 和 handler。
//!
//! 本模块属于 stock HTTP 层，模板只提供可选预设，不能禁止物品添加自定义属性。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{TemplateCopyRequest, TemplateFieldDef, TemplateFieldResponse, TemplateFieldType};
use crate::{
    http::{ValidatedJson, ValidatedPath},
    security::CurrentUser,
    state::CoreState,
    stock::service::{self, StockApiError},
    validation::{validate_not_blank, validate_optional_not_blank},
};

/// 物品模板字段单位模式。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ItemAttributeUnitMode {
    /// 字段不使用单位。
    None,
    /// 模板提供固定单位，物品录入时不可修改。
    Fixed,
    /// 模板提供单位候选项，物品录入时必须选择其中之一。
    Select,
    /// 物品录入时允许自由填写单位。
    Custom,
}

impl ItemAttributeUnitMode {
    /// 返回数据库保存的稳定代码。
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fixed => "fixed",
            Self::Select => "select",
            Self::Custom => "custom",
        }
    }

    /// 从数据库稳定代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "none" => Ok(Self::None),
            "fixed" => Ok(Self::Fixed),
            "select" => Ok(Self::Select),
            "custom" => Ok(Self::Custom),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 物品模板字段单位规则。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemAttributeUnitRule {
    /// 单位模式。
    #[garde(dive)]
    pub mode: ItemAttributeUnitMode,
    /// fixed 模式的固定单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub value: Option<String>,
    /// select 模式的单位候选项。
    #[garde(inner(length(min = 1, max = 32)))]
    pub options: Option<Vec<String>>,
}

/// 物品属性模板字段定义请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemAttributeTemplateFieldDef {
    /// 字段名称，同一模板内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
    /// 是否必填。
    #[garde(skip)]
    pub required: Option<bool>,
    /// 是否允许参与搜索。
    #[garde(skip)]
    pub searchable: Option<bool>,
    /// select 字段候选值。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,
    /// 可选默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
    /// 可选单位规则；缺失时按 none 处理。
    #[garde(dive)]
    pub unit: Option<ItemAttributeUnitRule>,
}

impl ItemAttributeTemplateFieldDef {
    /// 拆分两类模板共用字段和物品模板专属单位规则。
    pub(crate) fn into_parts(self) -> (TemplateFieldDef, Option<ItemAttributeUnitRule>) {
        (
            TemplateFieldDef {
                field_name: self.field_name,
                field_type: self.field_type,
                required: self.required,
                searchable: self.searchable,
                options: self.options,
                default_value: self.default_value,
            },
            self.unit,
        )
    }
}

/// 物品属性模板字段响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct ItemAttributeTemplateFieldResponse {
    /// 两类模板共用的基础字段信息。
    #[serde(flatten)]
    #[garde(dive)]
    pub field: TemplateFieldResponse,
    /// 单位规则。
    #[garde(dive)]
    pub unit: ItemAttributeUnitRule,
}

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
    pub fields: Vec<ItemAttributeTemplateFieldDef>,
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
    pub fields: Option<Vec<ItemAttributeTemplateFieldDef>>,
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
    pub fields: Vec<ItemAttributeTemplateFieldResponse>,
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
