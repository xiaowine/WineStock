//! 物品分类业务服务。
//!
//! 本模块属于 stock 服务层，只处理归类元数据，不把分类解释为属性模板。

use super::super::{
    validation::{normalize_optional_text, normalize_required_text},
    StockApiError,
};
use crate::{
    persistence::{
        entity::stock_item_category,
        repository::{CreateItemCategory, StockRepository, UpdateItemCategory},
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

fn response(
    model: stock_item_category::Model,
    item_usage_count: u64,
) -> controller::ItemCategoryResponse {
    controller::ItemCategoryResponse {
        id: model.id,
        name: model.name,
        description: model.description,
        sort_order: model.sort_order,
        item_usage_count,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}
/// 创建物品分类。
pub(crate) async fn create_item_category(
    state: &CoreState,
    user: &CurrentUser,
    request: controller::ItemCategoryCreateRequest,
) -> Result<controller::ItemCategoryResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_item_category_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::CategoryNameTaken);
    }
    Ok(response(
        repository
            .create_item_category(
                CreateItemCategory {
                    name,
                    description: normalize_optional_text(request.description)?,
                    sort_order: request.sort_order.unwrap_or(0),
                },
                Some(user.user_id),
            )
            .await?,
        0,
    ))
}
/// 查询物品分类列表。
pub(crate) async fn list_item_categories(
    state: &CoreState,
) -> Result<Vec<controller::ItemCategoryResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    let usage_counts = repository.active_item_category_usage_counts().await?;
    Ok(repository
        .list_active_item_categories()
        .await?
        .into_iter()
        .map(|model| {
            let usage_count = usage_counts.get(&model.id).copied().unwrap_or(0);
            response(model, usage_count)
        })
        .collect())
}
/// 查询物品分类详情。
pub(crate) async fn get_item_category(
    state: &CoreState,
    id: i64,
) -> Result<controller::ItemCategoryResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let usage_count = repository.active_item_category_usage_count(id).await?;
    repository
        .find_active_item_category_by_id(id)
        .await?
        .map(|model| response(model, usage_count))
        .ok_or(StockApiError::CategoryNotFound)
}
/// 更新物品分类。
pub(crate) async fn update_item_category(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
    request: controller::ItemCategoryUpdateRequest,
) -> Result<controller::ItemCategoryResponse, StockApiError> {
    let name = request
        .name
        .map(|value| normalize_required_text(&value))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(name) = name.as_deref() {
        if repository
            .active_item_category_name_exists_except(name, Some(id))
            .await?
        {
            return Err(StockApiError::CategoryNameTaken);
        }
    }
    let updated = repository
        .update_item_category(
            id,
            UpdateItemCategory {
                name,
                description: request
                    .description
                    .map(|value| normalize_required_text(&value))
                    .transpose()?
                    .map(Some),
                sort_order: request.sort_order,
            },
            Some(user.user_id),
        )
        .await?
        .ok_or(StockApiError::CategoryNotFound)?;
    let usage_count = repository.active_item_category_usage_count(id).await?;
    Ok(response(updated, usage_count))
}
/// 软删除物品分类。
pub(crate) async fn delete_item_category(
    state: &CoreState,
    user: &CurrentUser,
    id: i64,
) -> Result<controller::ItemCategoryDeleteResponse, StockApiError> {
    StockRepository::new(state.database())
        .soft_delete_item_category(id, Some(user.user_id))
        .await?
        .map(
            |affected_active_item_count| controller::ItemCategoryDeleteResponse {
                affected_active_item_count,
            },
        )
        .ok_or(StockApiError::CategoryNotFound)
}
