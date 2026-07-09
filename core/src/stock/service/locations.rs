//! 库位分组、库位和移库服务。
//!
//! 本模块属于 `stock` 业务服务层，负责库位树、库位主数据和整批次移库的业务校验。
//! 物品主数据不保存库位，当前库存位置只从批次读取和更新。

use crate::{
    persistence::repository::{
        CreateLocation, CreateLocationGroup, CreateLocationTransfer, StockLocationGroupRecord,
        StockLocationRecord, StockRepository, UpdateLocation, UpdateLocationGroup,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    response::{location_group_response, location_response, location_transfer_response},
    validation::{normalize_optional_text, normalize_required_text, positive_id},
    StockApiError,
};

/// 查询库位分组树；每个分组节点内包含直接子分组和直接库位。
pub(crate) async fn list_location_group_tree(
    state: &CoreState,
) -> Result<Vec<controller::LocationGroupTreeNode>, StockApiError> {
    let repository = StockRepository::new(state.database());
    let groups = repository.list_active_location_groups().await?;
    let locations = repository.list_active_locations(None, None).await?;

    Ok(build_group_tree(&groups, &locations, None))
}

/// 创建库位分组；同一父分组下名称不能重复。
pub(crate) async fn create_location_group(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::LocationGroupCreateRequest,
) -> Result<controller::LocationGroupResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let parent_id = normalize_parent_id(&repository, request.parent_id).await?;
    let name = normalize_required_text(&request.name)?;
    let sort_order = normalize_sort_order(request.sort_order)?;
    if repository
        .active_location_group_name_exists(parent_id, &name, None)
        .await?
    {
        return Err(StockApiError::LocationGroupNameTaken);
    }

    let group = repository
        .create_location_group(
            CreateLocationGroup {
                parent_id,
                name,
                sort_order,
            },
            Some(current_user.user_id),
        )
        .await?;

    Ok(location_group_response(group))
}

/// 更新库位分组；父级变更会记录为 `moved` 事件，普通字段变更记录为 `updated`。
pub(crate) async fn update_location_group(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::LocationGroupUpdateRequest,
) -> Result<controller::LocationGroupResponse, StockApiError> {
    let id = positive_id(id)?;
    let repository = StockRepository::new(state.database());
    if repository
        .find_active_location_group_by_id(id)
        .await?
        .is_none()
    {
        return Err(StockApiError::LocationGroupNotFound);
    }
    let parent_id = normalize_parent_id(&repository, request.parent_id).await?;
    if let Some(parent_id) = parent_id {
        if repository
            .location_group_has_descendant(id, parent_id)
            .await?
        {
            return Err(StockApiError::LocationGroupCycle);
        }
    }
    let name = normalize_required_text(&request.name)?;
    let sort_order = normalize_sort_order(request.sort_order)?;
    if repository
        .active_location_group_name_exists(parent_id, &name, Some(id))
        .await?
    {
        return Err(StockApiError::LocationGroupNameTaken);
    }

    let group = repository
        .update_location_group(
            id,
            UpdateLocationGroup {
                parent_id,
                name,
                sort_order,
            },
            Some(current_user.user_id),
        )
        .await?
        .ok_or(StockApiError::LocationGroupNotFound)?;

    Ok(location_group_response(group))
}

/// 删除空库位分组；仍有子分组或有效库位时拒绝删除。
pub(crate) async fn delete_location_group(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<(), StockApiError> {
    let id = positive_id(id)?;
    let repository = StockRepository::new(state.database());
    if repository
        .find_active_location_group_by_id(id)
        .await?
        .is_none()
    {
        return Err(StockApiError::LocationGroupNotFound);
    }
    if repository.location_group_has_children(id).await?
        || repository.location_group_has_locations(id).await?
    {
        return Err(StockApiError::LocationGroupInUse);
    }
    if repository
        .soft_delete_location_group(id, Some(current_user.user_id))
        .await?
    {
        Ok(())
    } else {
        Err(StockApiError::LocationGroupNotFound)
    }
}

/// 查询库位列表，可按分组或搜索词筛选。
pub(crate) async fn list_locations(
    state: &CoreState,
    query: controller::LocationListQuery,
) -> Result<Vec<controller::LocationResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    let group_id = match query.group_id {
        Some(group_id) => {
            let group_id = positive_id(group_id)?;
            if repository
                .find_active_location_group_by_id(group_id)
                .await?
                .is_none()
            {
                return Err(StockApiError::LocationGroupNotFound);
            }
            Some(group_id)
        }
        None => None,
    };
    let search = normalize_optional_text(query.search)?;
    let locations = repository
        .list_active_locations(group_id, search.as_deref())
        .await?
        .into_iter()
        .map(location_response)
        .collect::<Vec<_>>();

    Ok(locations)
}

/// 创建库位；库位编码在未软删除库位内全局唯一。
pub(crate) async fn create_location(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::LocationCreateRequest,
) -> Result<controller::LocationResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let group_id = positive_id(request.group_id)?;
    if repository
        .find_active_location_group_by_id(group_id)
        .await?
        .is_none()
    {
        return Err(StockApiError::LocationGroupNotFound);
    }
    let code = normalize_required_text(&request.code)?;
    if repository.active_location_code_exists(&code, None).await? {
        return Err(StockApiError::LocationCodeTaken);
    }
    let location = repository
        .create_location(
            CreateLocation {
                group_id,
                code,
                name: normalize_required_text(&request.name)?,
                sort_order: normalize_sort_order(request.sort_order)?,
            },
            Some(current_user.user_id),
        )
        .await?;

    Ok(location_response(location))
}

/// 更新库位基础资料；若改变分组，只影响后续库存展示，不改变历史单据。
pub(crate) async fn update_location(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::LocationUpdateRequest,
) -> Result<controller::LocationResponse, StockApiError> {
    let id = positive_id(id)?;
    let repository = StockRepository::new(state.database());
    if repository.find_active_location_by_id(id).await?.is_none() {
        return Err(StockApiError::LocationNotFound);
    }
    let group_id = positive_id(request.group_id)?;
    if repository
        .find_active_location_group_by_id(group_id)
        .await?
        .is_none()
    {
        return Err(StockApiError::LocationGroupNotFound);
    }
    let code = normalize_required_text(&request.code)?;
    if repository
        .active_location_code_exists(&code, Some(id))
        .await?
    {
        return Err(StockApiError::LocationCodeTaken);
    }
    let location = repository
        .update_location(
            id,
            UpdateLocation {
                group_id,
                code,
                name: normalize_required_text(&request.name)?,
                sort_order: normalize_sort_order(request.sort_order)?,
            },
            Some(current_user.user_id),
        )
        .await?
        .ok_or(StockApiError::LocationNotFound)?;

    Ok(location_response(location))
}

/// 删除库位；已有历史单据允许保留引用，但仍有当前库存时禁止删除。
pub(crate) async fn delete_location(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<(), StockApiError> {
    let id = positive_id(id)?;
    let repository = StockRepository::new(state.database());
    if repository.find_active_location_by_id(id).await?.is_none() {
        return Err(StockApiError::LocationNotFound);
    }
    if repository.location_has_current_stock(id).await? {
        return Err(StockApiError::LocationInUse);
    }
    if repository
        .soft_delete_location(id, Some(current_user.user_id))
        .await?
    {
        Ok(())
    } else {
        Err(StockApiError::LocationNotFound)
    }
}

/// 整批次移库；只移动仍有余额的批次，且调用方必须带上当前原库位 ID。
pub(crate) async fn create_location_transfer(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::LocationTransferCreateRequest,
) -> Result<controller::LocationTransferResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let transfer = repository
        .create_location_transfer(CreateLocationTransfer {
            batch_id: positive_id(request.batch_id)?,
            from_location_id: positive_id(request.from_location_id)?,
            to_location_id: positive_id(request.to_location_id)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
        })
        .await
        .map_err(map_stock_db_error)?;

    Ok(location_transfer_response(transfer))
}

async fn normalize_parent_id(
    repository: &StockRepository<'_>,
    parent_id: Option<i64>,
) -> Result<Option<i64>, StockApiError> {
    let Some(parent_id) = parent_id else {
        return Ok(None);
    };
    let parent_id = positive_id(parent_id)?;
    if repository
        .find_active_location_group_by_id(parent_id)
        .await?
        .is_none()
    {
        return Err(StockApiError::LocationGroupNotFound);
    }

    Ok(Some(parent_id))
}

fn normalize_sort_order(value: Option<i32>) -> Result<i32, StockApiError> {
    let value = value.unwrap_or(0);
    if value < 0 {
        Err(StockApiError::InvalidRequest)
    } else {
        Ok(value)
    }
}

fn build_group_tree(
    groups: &[StockLocationGroupRecord],
    locations: &[StockLocationRecord],
    parent_id: Option<i64>,
) -> Vec<controller::LocationGroupTreeNode> {
    groups
        .iter()
        .filter(|group| group.parent_id == parent_id)
        .map(|group| controller::LocationGroupTreeNode {
            id: group.id,
            parent_id: group.parent_id,
            name: group.name.clone(),
            sort_order: group.sort_order,
            created_at: group.created_at.clone(),
            updated_at: group.updated_at.clone(),
            locations: locations
                .iter()
                .filter(|location| location.group_id == group.id)
                .cloned()
                .map(location_response)
                .collect(),
            children: build_group_tree(groups, locations, Some(group.id)),
        })
        .collect()
}
