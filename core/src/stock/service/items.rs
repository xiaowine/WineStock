//! 库存物品服务。
//!
//! 本模块属于 `stock` 业务服务层，负责物品创建、分页、筛选值、详情、更新、软删除、SKU 冲突检查和审计操作者传递。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use crate::{
    files::stored_image_matches_metadata,
    persistence::repository::{
        CatalogSort, CatalogStockFilter, CreateStockItem, FileObjectRepository,
        ItemCatalogCriteria, ItemOptionCriteria, StockRepository, UpdateStockItem,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    item_attributes::normalize_item_attributes,
    pagination::{total_pages, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{
        filter_values_response, item_catalog_response, item_editor_response,
        item_inventory_response, item_option_response,
    },
    validation::{normalize_optional_text, normalize_required_text, validate_non_negative},
    StockApiError,
};

/// 创建库存物品；会写入未软删除物品记录、检查 SKU 唯一性，并记录创建审计事件。
pub(crate) async fn create_item(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::ItemCreateRequest,
) -> Result<controller::ItemMutationResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    validate_item_main_image(state, current_user, request.image_file_id).await?;
    if let Some(category_id) = request.category_id {
        if repository
            .find_active_item_category_by_id(category_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::CategoryNotFound);
        }
    }
    let attributes = normalize_item_attributes(
        &repository,
        current_user,
        request.attribute_template_id,
        None,
        request.attributes,
    )
    .await?;
    if attributes
        .iter()
        .any(|attribute| attribute.file_object_id == Some(request.image_file_id))
    {
        return Err(StockApiError::ItemImageUnavailable {
            file_id: request.image_file_id,
        });
    }
    let input = CreateStockItem {
        name: normalize_required_text(&request.name)?,
        sku: normalize_required_text(&request.sku)?,
        category_id: request.category_id,
        attribute_template_id: request.attribute_template_id,
        image_file_id: request.image_file_id,
        image_owner_user_id: current_user.user_id,
        unit: normalize_required_text(&request.unit)?,
        description: normalize_optional_text(request.description)?,
        default_price: validate_non_negative(request.default_price)?,
        reorder_point: validate_non_negative(request.reorder_point)?,
        attributes,
    };
    if repository
        .active_sku_exists_except(&input.sku, None)
        .await?
    {
        return Err(StockApiError::SkuTaken);
    }

    let item = repository
        .create_item(input, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?;
    Ok(controller::ItemMutationResponse {
        id: item.id,
        updated_at: item.updated_at,
    })
}

/// 查询物品目录库存视图；HTTP 默认值在服务层归一化为仓储领域条件。
pub(crate) async fn list_item_catalog(
    state: &CoreState,
    query: controller::ItemCatalogQuery,
) -> Result<controller::ItemCatalogPageResponse, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let search = normalize_optional_text(query.search)?;
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_item_catalog(ItemCatalogCriteria {
            page,
            page_size,
            search,
            category_id: query.category_id,
            attribute_template_id: query.attribute_template_id,
            stock_filter: match query
                .stock_filter
                .unwrap_or(controller::ItemStockFilter::All)
            {
                controller::ItemStockFilter::All => CatalogStockFilter::All,
                controller::ItemStockFilter::NeedsAttention => CatalogStockFilter::NeedsAttention,
                controller::ItemStockFilter::OutOfStock => CatalogStockFilter::OutOfStock,
                controller::ItemStockFilter::ReorderDue => CatalogStockFilter::ReorderDue,
                controller::ItemStockFilter::NeedsConfiguration => {
                    CatalogStockFilter::NeedsConfiguration
                }
            },
            sort: match query
                .sort
                .unwrap_or(controller::ItemCatalogSort::ReplenishmentPriority)
            {
                controller::ItemCatalogSort::ReplenishmentPriority => {
                    CatalogSort::ReplenishmentPriority
                }
                controller::ItemCatalogSort::Name => CatalogSort::Name,
                controller::ItemCatalogSort::QuantityAsc => CatalogSort::QuantityAsc,
                controller::ItemCatalogSort::QuantityDesc => CatalogSort::QuantityDesc,
                controller::ItemCatalogSort::InventoryValueDesc => CatalogSort::InventoryValueDesc,
                controller::ItemCatalogSort::UpdatedDesc => CatalogSort::UpdatedDesc,
            },
        })
        .await?;
    item_catalog_response(result, page, page_size)
}

/// 查询入库等业务选择器使用的轻量物品分页。
pub(crate) async fn list_item_options(
    state: &CoreState,
    query: controller::ItemOptionQuery,
) -> Result<controller::ItemOptionPageResponse, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_item_options(ItemOptionCriteria {
            page,
            page_size,
            search: normalize_optional_text(query.search)?,
            category_id: query.category_id,
            attribute_template_id: query.attribute_template_id,
        })
        .await?;
    Ok(controller::ItemOptionPageResponse {
        items: result.items.into_iter().map(item_option_response).collect(),
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
) -> Result<controller::ItemEditorResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_active_item_editor_by_id(id).await? else {
        return Err(StockApiError::ItemNotFound);
    };
    item_editor_response(detail)
}

/// 查询已有物品库存摘要和库位分布，不加载批次明细。
pub(crate) async fn get_item_inventory(
    state: &CoreState,
    id: i64,
) -> Result<controller::ItemInventoryResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_item_inventory_by_id(id).await? else {
        return Err(StockApiError::ItemNotFound);
    };
    item_inventory_response(detail)
}

/// 分页查询单个物品当前有效批次。
pub(crate) async fn list_item_batches(
    state: &CoreState,
    id: i64,
    query: controller::ItemBatchQuery,
) -> Result<controller::ItemBatchPageResponse, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let repository = StockRepository::new(state.database());
    if repository.find_active_item_by_id(id).await?.is_none() {
        return Err(StockApiError::ItemNotFound);
    }
    let result = repository
        .list_item_stock_batches(id, page, page_size)
        .await?;
    Ok(controller::ItemBatchPageResponse {
        items: result
            .items
            .into_iter()
            .map(|batch| controller::ItemBatchStockResponse {
                id: batch.id,
                batch_no: batch.batch_no,
                location_id: batch.location_id,
                location_code: batch.location_code,
                location_name: batch.location_name,
                initial_quantity: batch.initial_quantity,
                remaining_quantity: batch.remaining_quantity,
                unit_cost: batch.unit_cost,
                value: batch.value,
                received_at: batch.received_at,
                expires_at: batch.expires_at,
            })
            .collect(),
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 更新库存物品基础资料；字段缺失表示不修改，显式 null 可清空对应可空字段。
///
/// 成功更新时会记录物品审计事件，包含关键字段的前后快照。
pub(crate) async fn update_item(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::ItemUpdateRequest,
) -> Result<controller::ItemMutationResponse, StockApiError> {
    let sku = request
        .sku
        .map(|sku| normalize_required_text(&sku))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    let current = repository
        .find_active_item_by_id(id)
        .await?
        .ok_or(StockApiError::ItemNotFound)?;
    let image_file_id = request
        .image_file_id
        .filter(|file_id| *file_id != current.image_file_id);
    if let Some(file_id) = image_file_id {
        validate_item_main_image(state, current_user, file_id).await?;
    }
    if let Some(sku) = sku.as_deref() {
        if repository.active_sku_exists_except(sku, Some(id)).await? {
            return Err(StockApiError::SkuTaken);
        }
    }

    if let Some(Some(category_id)) = request.category_id {
        if repository
            .find_active_item_category_by_id(category_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::CategoryNotFound);
        }
    }
    let effective_template_id = request
        .attribute_template_id
        .unwrap_or(current.attribute_template_id);
    if request.attribute_template_id.is_some()
        && effective_template_id != current.attribute_template_id
        && request.attributes.is_none()
    {
        return Err(StockApiError::InvalidRequest);
    }
    let attributes = match request.attributes {
        Some(attributes) => Some(
            normalize_item_attributes(
                &repository,
                current_user,
                effective_template_id,
                Some(id),
                attributes,
            )
            .await?,
        ),
        None => None,
    };
    let effective_image_file_id = image_file_id.unwrap_or(current.image_file_id);
    if attributes.as_ref().is_some_and(|attributes| {
        attributes
            .iter()
            .any(|attribute| attribute.file_object_id == Some(effective_image_file_id))
    }) {
        return Err(StockApiError::ItemImageUnavailable {
            file_id: effective_image_file_id,
        });
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
                category_id: request.category_id,
                attribute_template_id: request.attribute_template_id,
                image_file_id,
                image_owner_user_id: image_file_id.map(|_| current_user.user_id),
                unit: request
                    .unit
                    .map(|unit| normalize_required_text(&unit))
                    .transpose()?,
                description: normalize_nullable_text(request.description)?,
                default_price: validate_nullable_non_negative(request.default_price)?,
                reorder_point: validate_nullable_non_negative(request.reorder_point)?,
                attributes,
            },
            Some(current_user.user_id),
        )
        .await
        .map_err(map_stock_db_error)?
    else {
        return Err(StockApiError::ItemNotFound);
    };

    Ok(controller::ItemMutationResponse {
        id: item.id,
        updated_at: item.updated_at,
    })
}

/// 校验物品主图文件元数据、磁盘内容、所有权和当前绑定状态。
async fn validate_item_main_image(
    state: &CoreState,
    current_user: &CurrentUser,
    file_id: i64,
) -> Result<(), StockApiError> {
    let record = FileObjectRepository::new(state.database())
        .find_access_record(file_id)
        .await?
        .ok_or(StockApiError::ItemImageUnavailable { file_id })?;
    let owned_unbound =
        record.file.owner_user_id == Some(current_user.user_id) && !record.is_bound();
    if !owned_unbound || !stored_image_matches_metadata(state.storage(), &record.file) {
        return Err(StockApiError::ItemImageUnavailable { file_id });
    }
    Ok(())
}

fn normalize_nullable_text(
    value: Option<Option<String>>,
) -> Result<Option<Option<String>>, StockApiError> {
    value
        .map(|value| value.map(|text| normalize_required_text(&text)).transpose())
        .transpose()
}

fn validate_nullable_non_negative(
    value: Option<Option<f64>>,
) -> Result<Option<Option<f64>>, StockApiError> {
    value
        .map(|value| {
            value
                .map(|number| validate_non_negative(Some(number)))
                .transpose()
        })
        .transpose()
        .map(|value| value.map(|value| value.flatten()))
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
