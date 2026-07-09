//! 库存物品服务。
//!
//! 本模块属于 `stock` 业务服务层，负责物品创建、分页、筛选值、详情、更新、软删除、SKU 冲突检查和审计操作者传递。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use crate::{
    persistence::repository::{CreateStockItem, ListStockItems, StockRepository, UpdateStockItem},
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{filter_values_response, item_detail_response, item_response},
    validation::{normalize_optional_text, normalize_required_text, validate_non_negative},
    StockApiError,
};

/// 创建库存物品；会写入未软删除物品记录、检查 SKU 唯一性，并记录创建审计事件。
pub(crate) async fn create_item(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::ItemCreateRequest,
) -> Result<controller::ItemResponse, StockApiError> {
    let input = CreateStockItem {
        name: normalize_required_text(&request.name)?,
        sku: normalize_required_text(&request.sku)?,
        category_id: request.category_id,
        unit: normalize_required_text(&request.unit)?,
        description: normalize_optional_text(request.description)?,
        default_price: validate_non_negative(request.default_price)?,
        reorder_point: validate_non_negative(request.reorder_point)?,
    };
    let repository = StockRepository::new(state.database());
    if repository
        .active_sku_exists_except(&input.sku, None)
        .await?
    {
        return Err(StockApiError::SkuTaken);
    }

    Ok(item_response(
        repository
            .create_item(input, Some(current_user.user_id))
            .await?,
    ))
}

/// 分页查询库存物品；查询参数在这里统一归一化，避免 repository 暴露 HTTP 默认值。
pub(crate) async fn list_items(
    state: &CoreState,
    query: controller::ItemListQuery,
) -> Result<PaginatedResponse<controller::ItemResponse>, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let search = normalize_optional_text(query.search)?;
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_active_items(ListStockItems {
            page,
            page_size,
            search,
            category_id: query.category_id,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result.items.into_iter().map(item_response).collect(),
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询当前库存视角下的物品筛选值；只返回有库存批次贡献出的值。
pub(crate) async fn item_filter_values(
    state: &CoreState,
) -> Result<controller::FilterValuesResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    filter_values_response(repository.list_item_filter_values().await?)
}

/// 查询单个库存物品详情；返回未软删除物品的基础资料和当前库存快照。
pub(crate) async fn get_item(
    state: &CoreState,
    id: i64,
) -> Result<controller::ItemDetailResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_active_item_detail_by_id(id).await? else {
        return Err(StockApiError::ItemNotFound);
    };

    Ok(item_detail_response(detail))
}

/// 更新库存物品基础资料；字段为空表示不修改，当前接口不通过 null 清空可空字段。
///
/// 成功更新时会记录物品审计事件，包含关键字段的前后快照。
pub(crate) async fn update_item(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::ItemUpdateRequest,
) -> Result<controller::ItemResponse, StockApiError> {
    let sku = request
        .sku
        .map(|sku| normalize_required_text(&sku))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(sku) = sku.as_deref() {
        if repository.active_sku_exists_except(sku, Some(id)).await? {
            return Err(StockApiError::SkuTaken);
        }
    }

    let Some(item) = repository
        .update_item(
            id,
            UpdateStockItem {
                name: request
                    .name
                    .map(|name| normalize_required_text(&name))
                    .transpose()?,
                sku,
                category_id: request.category_id.map(Some),
                unit: request
                    .unit
                    .map(|unit| normalize_required_text(&unit))
                    .transpose()?,
                description: request
                    .description
                    .map(|description| normalize_required_text(&description))
                    .transpose()?
                    .map(Some),
                default_price: request
                    .default_price
                    .map(|value| {
                        validate_non_negative(Some(value)).map(|value| value.expect("输入值已存在"))
                    })
                    .transpose()?
                    .map(Some),
                reorder_point: request
                    .reorder_point
                    .map(|value| {
                        validate_non_negative(Some(value)).map(|value| value.expect("输入值已存在"))
                    })
                    .transpose()?
                    .map(Some),
            },
            Some(current_user.user_id),
        )
        .await?
    else {
        return Err(StockApiError::ItemNotFound);
    };

    Ok(item_response(item))
}

/// 软删除库存物品；删除后物品不会再出现在库存物品查询结果中，并记录删除审计事件。
pub(crate) async fn delete_item(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<(), StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository
        .soft_delete_item(id, Some(current_user.user_id))
        .await?
    {
        Ok(())
    } else {
        Err(StockApiError::ItemNotFound)
    }
}
