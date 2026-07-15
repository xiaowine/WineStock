//! 入库属性模板 HTTP DTO 和 handler。
//!
//! 本模块属于 stock HTTP 层，只暴露描述单次收货状态的模板接口。

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

/// 创建入库模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundTemplateCreateRequest {
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 本次收货属性字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldDef>,
}

/// 更新入库模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundTemplateUpdateRequest {
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 字段存在时整体替换。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldDef>>,
}

/// 入库模板响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct InboundTemplateResponse {
    /// 模板 ID。
    #[garde(skip)]
    pub id: i64,
    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 本次收货属性字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldResponse>,
    /// 创建时间。
    #[garde(skip)]
    pub created_at: String,
    /// 更新时间。
    #[garde(skip)]
    pub updated_at: String,
}

/// 创建入库模板。
#[utoipa::path(post, path = "/api/inbound-templates", tag = "inbound-templates", request_body = InboundTemplateCreateRequest, security(("bearerAuth" = [])), responses((status = 201, body = InboundTemplateResponse)))]
pub(crate) async fn create_inbound_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<InboundTemplateCreateRequest>,
) -> Result<(StatusCode, Json<InboundTemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_inbound_template(&state, &user, request).await?),
    ))
}
/// 查询入库模板列表。
///
/// 允许 `stock.inbound.create` 或 `stock.template.read` 任一权限，
/// 让入库任务在不开放模板管理权限的情况下读取入库模板候选。
#[utoipa::path(
    get,
    path = "/api/inbound-templates",
    tag = "inbound-templates",
    summary = "查询入库模板列表。",
    description = "允许 stock.inbound.create 或 stock.template.read 任一权限；仅用于入库任务读取活动模板，写接口仍要求 stock.template.manage。",
    security(("bearerAuth" = [])),
    responses((status = 200, body = Vec<InboundTemplateResponse>))
)]
pub(crate) async fn list_inbound_templates(
    State(state): State<CoreState>,
) -> Result<Json<Vec<InboundTemplateResponse>>, StockApiError> {
    Ok(Json(service::list_inbound_templates(&state).await?))
}
/// 查询入库模板详情。
///
/// 允许 `stock.inbound.create` 或 `stock.template.read` 任一权限，
/// 与列表接口保持一致的任务内读取例外。
#[utoipa::path(
    get,
    path = "/api/inbound-templates/{id}",
    tag = "inbound-templates",
    summary = "查询入库模板详情。",
    description = "允许 stock.inbound.create 或 stock.template.read 任一权限；仅用于入库任务读取活动模板，写接口仍要求 stock.template.manage。",
    params(("id" = i64, Path)),
    security(("bearerAuth" = [])),
    responses((status = 200, body = InboundTemplateResponse))
)]
pub(crate) async fn get_inbound_template(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<InboundTemplateResponse>, StockApiError> {
    Ok(Json(service::get_inbound_template(&state, id).await?))
}
/// 更新入库模板。
#[utoipa::path(put, path = "/api/inbound-templates/{id}", tag = "inbound-templates", params(("id" = i64, Path)), request_body = InboundTemplateUpdateRequest, security(("bearerAuth" = [])), responses((status = 200, body = InboundTemplateResponse)))]
pub(crate) async fn update_inbound_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<InboundTemplateUpdateRequest>,
) -> Result<Json<InboundTemplateResponse>, StockApiError> {
    Ok(Json(
        service::update_inbound_template(&state, &user, id, request).await?,
    ))
}
/// 软删除入库模板。
#[utoipa::path(delete, path = "/api/inbound-templates/{id}", tag = "inbound-templates", params(("id" = i64, Path)), security(("bearerAuth" = [])), responses((status = 204)))]
pub(crate) async fn delete_inbound_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_inbound_template(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
/// 复制入库模板。
#[utoipa::path(post, path = "/api/inbound-templates/{id}/copy", tag = "inbound-templates", params(("id" = i64, Path)), request_body = TemplateCopyRequest, security(("bearerAuth" = [])), responses((status = 201, body = InboundTemplateResponse)))]
pub(crate) async fn copy_inbound_template(
    State(state): State<CoreState>,
    Extension(user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<TemplateCopyRequest>,
) -> Result<(StatusCode, Json<InboundTemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::copy_inbound_template(&state, &user, id, request).await?),
    ))
}
