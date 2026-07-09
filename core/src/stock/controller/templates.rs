//! 库存模板 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责模板相关请求、响应和 Axum 入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, state::CoreState};

use crate::stock::service::{self, StockApiError};
/// 模板字段类型。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TemplateFieldType {
    /// 普通文本字段。
    Text,

    /// 数值字段。
    Number,

    /// 预置选项字段。
    Select,

    /// 日期字段，值使用日期字符串。
    Date,

    /// 文件字段，值引用文件元数据。
    File,

    /// 网页链接字段，值必须是 HTTP 或 HTTPS URL。
    Url,

    /// 布尔字段，默认值只允许 `true` 或 `false`。
    Boolean,
}

impl TemplateFieldType {
    /// 返回数据库中保存的稳定字段类型代码。
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Select => "select",
            Self::Date => "date",
            Self::File => "file",
            Self::Url => "url",
            Self::Boolean => "boolean",
        }
    }

    /// 从数据库字段类型代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "select" => Ok(Self::Select),
            "date" => Ok(Self::Date),
            "file" => Ok(Self::File),
            "url" => Ok(Self::Url),
            "boolean" => Ok(Self::Boolean),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 模板字段定义请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateFieldDef {
    /// 字段名称，同一模板内必须唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,

    /// 是否必填；未传时默认为 false。
    #[garde(skip)]
    pub required: Option<bool>,

    /// 是否可用于搜索；未传时默认为 false。
    #[garde(skip)]
    pub searchable: Option<bool>,

    /// `select` 字段的候选值；其他字段类型不允许传入。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,

    /// 默认值；数值、布尔和选项字段会执行额外业务校验。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
}

/// 创建库存模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCreateRequest {
    /// 模板名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义列表，至少一个字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldDef>,
}

/// 更新库存模板请求；字段为空表示不修改。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateUpdateRequest {
    /// 模板名称，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 模板说明；当前首版接口不通过 null 清空该字段。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义；存在时整体替换旧字段。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldDef>>,
}

/// 复制模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCopyRequest {
    /// 新模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
}

/// 模板字段响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct TemplateFieldResponse {
    /// 字段 ID。
    #[garde(skip)]
    pub id: i64,

    /// 字段名称。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,

    /// 是否必填。
    #[garde(skip)]
    pub required: bool,

    /// 是否可用于搜索。
    #[garde(skip)]
    pub searchable: bool,

    /// `select` 字段的候选值。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,

    /// 默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,

    /// 字段排序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 库存模板响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct TemplateResponse {
    /// 模板 ID。
    #[garde(skip)]
    pub id: i64,

    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldResponse>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,
}

#[utoipa::path(
    post,
    path = "/api/templates",
    tag = "stock",
    request_body = TemplateCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Template created", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 创建库存模板。
pub(crate) async fn create_template(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<TemplateCreateRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_template(&state, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "stock",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template list", body = Vec<TemplateResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template read permission required", body = String)
    )
)]
/// 查询库存模板列表。
pub(crate) async fn list_templates(
    State(state): State<CoreState>,
) -> Result<Json<Vec<TemplateResponse>>, StockApiError> {
    Ok(Json(service::list_templates(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/templates/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template detail", body = TemplateResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template read permission required", body = String),
        (status = 404, description = "Template not found", body = String)
    )
)]
/// 查询单个库存模板。
pub(crate) async fn get_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<TemplateResponse>, StockApiError> {
    Ok(Json(service::get_template(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/templates/{id}",
    tag = "stock",
    request_body = TemplateUpdateRequest,
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template updated", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 更新库存模板。
pub(crate) async fn update_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<TemplateUpdateRequest>,
) -> Result<Json<TemplateResponse>, StockApiError> {
    Ok(Json(service::update_template(&state, id, request).await?))
}

#[utoipa::path(
    delete,
    path = "/api/templates/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template is referenced by active items", body = String)
    )
)]
/// 软删除库存模板。
pub(crate) async fn delete_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_template(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/templates/{id}/copy",
    tag = "stock",
    request_body = TemplateCopyRequest,
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Template copied", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 复制库存模板。
pub(crate) async fn copy_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<TemplateCopyRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::copy_template(&state, id, request).await?),
    ))
}
