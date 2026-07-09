//! 替代料服务。
//!
//! 本模块属于 `stock` 业务服务层，负责独立替代料 API 的整体替换、查询和删除关系用例。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use crate::{
    persistence::repository::{StockRepository, StockSubstituteInput},
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    response::{substitute_relation_response, substitute_response},
    validation::{normalize_optional_text, positive_i32, positive_id},
    StockApiError,
};

/// 整体替换指定物品的替代料列表；会写入替代料关系并记录审计事件。
///
/// 自引用、重复替代料、重复优先级和循环绑定会映射为 `InvalidRequest`。
pub(crate) async fn replace_substitutes(
    state: &CoreState,
    current_user: &CurrentUser,
    item_id: i64,
    request: controller::SubstituteReplaceRequest,
) -> Result<Vec<controller::ItemSubstituteResponse>, StockApiError> {
    let substitutes = request
        .substitutes
        .into_iter()
        .map(|substitute| {
            Ok(StockSubstituteInput {
                substitute_item_id: positive_id(substitute.substitute_item_id)?,
                priority: positive_i32(substitute.priority)?,
                notes: normalize_optional_text(substitute.notes)?,
            })
        })
        .collect::<Result<Vec<_>, StockApiError>>()?;
    let repository = StockRepository::new(state.database());
    let records = repository
        .replace_substitutes(item_id, substitutes, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::ItemNotFound)?;

    Ok(records.into_iter().map(substitute_response).collect())
}

/// 查询指定物品的替代料列表；主物品不存在或已软删除时返回 `ItemNotFound`。
pub(crate) async fn list_item_substitutes(
    state: &CoreState,
    item_id: i64,
) -> Result<Vec<controller::ItemSubstituteResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.find_active_item_by_id(item_id).await?.is_none() {
        return Err(StockApiError::ItemNotFound);
    }

    Ok(repository
        .list_item_substitutes(item_id)
        .await?
        .into_iter()
        .map(substitute_response)
        .collect())
}

/// 查询全部替代料关系；只返回未软删除的主物品和替代物品。
pub(crate) async fn list_substitute_relations(
    state: &CoreState,
) -> Result<Vec<controller::SubstituteRelationResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());

    Ok(repository
        .list_substitute_relations()
        .await?
        .into_iter()
        .map(substitute_relation_response)
        .collect())
}

/// 删除单个替代料关系；成功时会写入审计事件。
pub(crate) async fn delete_substitute_relation(
    state: &CoreState,
    current_user: &CurrentUser,
    item_id: i64,
    substitute_item_id: i64,
) -> Result<(), StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.find_active_item_by_id(item_id).await?.is_none() {
        return Err(StockApiError::ItemNotFound);
    }
    if !repository
        .delete_substitute_relation(item_id, substitute_item_id, Some(current_user.user_id))
        .await?
    {
        return Err(StockApiError::SubstituteNotFound);
    }

    Ok(())
}
