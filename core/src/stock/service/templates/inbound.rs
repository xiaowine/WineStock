//! 入库属性模板业务服务。
//!
//! 本模块属于 stock 服务层，只管理本次收货字段定义，不读取或修改物品固有属性。

use super::super::{
    validation::{normalize_optional_text, normalize_required_text},
    StockApiError,
};
use super::common::{inbound_template_response, normalize_template_fields};
use crate::{
    persistence::repository::{CreateInboundTemplate, StockRepository, UpdateInboundTemplate},
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

/// 创建入库属性模板。
pub(crate) async fn create_inbound_template(
    state: &CoreState,
    user: &CurrentUser,
    request: controller::InboundTemplateCreateRequest,
) -> Result<controller::InboundTemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_inbound_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }
    inbound_template_response(
        repository
            .create_inbound_template(
                CreateInboundTemplate {
                    name,
                    description: normalize_optional_text(request.description)?,
                    fields: normalize_template_fields(request.fields)?,
                },
                Some(user.user_id),
            )
            .await?,
    )
}
/// 查询全部有效入库模板。
pub(crate) async fn list_inbound_templates(
    state: &CoreState,
) -> Result<Vec<controller::InboundTemplateResponse>, StockApiError> {
    StockRepository::new(state.database())
        .list_active_inbound_templates()
        .await?
        .into_iter()
        .map(inbound_template_response)
        .collect()
}
/// 查询入库模板详情。
pub(crate) async fn get_inbound_template(
    state: &CoreState,
    id: i64,
) -> Result<controller::InboundTemplateResponse, StockApiError> {
    let Some(detail) = StockRepository::new(state.database())
        .find_active_inbound_template_by_id(id)
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    inbound_template_response(detail)
}
/// 更新入库模板。
pub(crate) async fn update_inbound_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
    request: controller::InboundTemplateUpdateRequest,
) -> Result<controller::InboundTemplateResponse, StockApiError> {
    let name = request
        .name
        .map(|value| normalize_required_text(&value))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(name) = name.as_deref() {
        if repository
            .active_inbound_template_name_exists_except(name, Some(id))
            .await?
        {
            return Err(StockApiError::TemplateNameTaken);
        }
    }
    let Some(detail) = repository
        .update_inbound_template(
            id,
            UpdateInboundTemplate {
                name,
                description: request
                    .description
                    .map(|value| normalize_required_text(&value))
                    .transpose()?
                    .map(Some),
                fields: request.fields.map(normalize_template_fields).transpose()?,
            },
            Some(user.user_id),
        )
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    inbound_template_response(detail)
}
/// 软删除入库模板。
pub(crate) async fn delete_inbound_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
) -> Result<(), StockApiError> {
    if StockRepository::new(state.database())
        .soft_delete_inbound_template(id, Some(user.user_id))
        .await?
    {
        Ok(())
    } else {
        Err(StockApiError::TemplateNotFound)
    }
}
/// 复制入库模板。
pub(crate) async fn copy_inbound_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
    request: controller::TemplateCopyRequest,
) -> Result<controller::InboundTemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_inbound_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }
    let Some(detail) = repository
        .copy_inbound_template(id, name, Some(user.user_id))
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    inbound_template_response(detail)
}
