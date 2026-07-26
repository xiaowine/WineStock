//! 物品属性模板业务服务。
//!
//! 本模块属于 stock 服务层，模板是可选录入预设，物品仍可保存任意自定义属性。

use super::super::{
    validation::{normalize_optional_text, normalize_required_text},
    StockApiError,
};
use super::common::{item_attribute_template_response, normalize_item_template_fields};
use crate::{
    persistence::repository::{
        CreateItemAttributeTemplate, StockRepository, UpdateItemAttributeTemplate,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

/// 创建物品属性模板。
pub(crate) async fn create_item_attribute_template(
    state: &CoreState,
    user: &CurrentUser,
    request: controller::ItemAttributeTemplateCreateRequest,
) -> Result<controller::ItemAttributeTemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_item_attribute_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }
    item_attribute_template_response(
        repository
            .create_item_attribute_template(
                CreateItemAttributeTemplate {
                    name,
                    description: normalize_optional_text(request.description)?,
                    fields: normalize_item_template_fields(request.fields)?,
                },
                Some(user.user_id),
            )
            .await?,
        0,
    )
}
/// 查询全部有效物品属性模板。
pub(crate) async fn list_item_attribute_templates(
    state: &CoreState,
) -> Result<Vec<controller::ItemAttributeTemplateResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    let usage_counts = repository
        .active_item_attribute_template_usage_counts()
        .await?;
    repository
        .list_active_item_attribute_templates()
        .await?
        .into_iter()
        .map(|detail| {
            let usage_count = usage_counts.get(&detail.template.id).copied().unwrap_or(0);
            item_attribute_template_response(detail, usage_count)
        })
        .collect()
}
/// 查询物品属性模板详情。
pub(crate) async fn get_item_attribute_template(
    state: &CoreState,
    id: i64,
) -> Result<controller::ItemAttributeTemplateResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let usage_count = repository
        .active_item_attribute_template_usage_count(id)
        .await?;
    let Some(detail) = repository
        .find_active_item_attribute_template_by_id(id)
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    item_attribute_template_response(detail, usage_count)
}
/// 更新物品属性模板。
pub(crate) async fn update_item_attribute_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
    request: controller::ItemAttributeTemplateUpdateRequest,
) -> Result<controller::ItemAttributeTemplateResponse, StockApiError> {
    let name = request
        .name
        .map(|value| normalize_required_text(&value))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(name) = name.as_deref() {
        if repository
            .active_item_attribute_template_name_exists_except(name, Some(id))
            .await?
        {
            return Err(StockApiError::TemplateNameTaken);
        }
    }
    let Some(detail) = repository
        .update_item_attribute_template(
            id,
            UpdateItemAttributeTemplate {
                name,
                description: request
                    .description
                    .map(|value| normalize_required_text(&value))
                    .transpose()?
                    .map(Some),
                fields: request
                    .fields
                    .map(normalize_item_template_fields)
                    .transpose()?,
                is_default: request.is_default,
            },
            Some(user.user_id),
        )
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    let usage_count = repository
        .active_item_attribute_template_usage_count(id)
        .await?;
    item_attribute_template_response(detail, usage_count)
}
/// 删除属性模板并清空使用物品的模板引用。
pub(crate) async fn delete_item_attribute_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
) -> Result<controller::ItemAttributeTemplateDeleteResponse, StockApiError> {
    StockRepository::new(state.database())
        .soft_delete_item_attribute_template(id, Some(user.user_id))
        .await?
        .map(
            |affected_active_item_count| controller::ItemAttributeTemplateDeleteResponse {
                affected_active_item_count,
            },
        )
        .ok_or(StockApiError::TemplateNotFound)
}
/// 复制物品属性模板。
pub(crate) async fn copy_item_attribute_template(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
    request: controller::TemplateCopyRequest,
) -> Result<controller::ItemAttributeTemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_item_attribute_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }
    let Some(detail) = repository
        .copy_item_attribute_template(id, name, Some(user.user_id))
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };
    item_attribute_template_response(detail, 0)
}
