//! 出库单服务。
//!
//! 本模块属于 `stock` 业务服务层，负责出库单创建、分页搜索、筛选值、详情、审批和拒绝。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use crate::{
    persistence::repository::{
        CreateOutboundOrder, CreateOutboundOrderItem, ListOutboundOrders, StockRepository,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{filter_values_response, outbound_response},
    validation::{normalize_optional_text, normalize_required_text, validate_positive},
    StockApiError,
};

/// 创建 pending 出库单；创建阶段只保存单据和明细，不扣减库存。
///
/// 库存充足性和指定批次可扣减性由审批阶段的事务校验。
pub(crate) async fn create_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::OutboundCreateRequest,
) -> Result<controller::OutboundResponse, StockApiError> {
    if request.items.is_empty() || request.items.len() > 256 {
        return Err(StockApiError::InvalidRequest);
    }
    let repository = StockRepository::new(state.database());
    let mut items = Vec::with_capacity(request.items.len());
    for item in request.items {
        if repository
            .find_active_item_by_id(item.item_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::ItemNotFound);
        }
        if item.batch_id.is_some_and(|id| id < 1) {
            return Err(StockApiError::InvalidRequest);
        }
        if item.location_id.is_some_and(|id| id < 1) {
            return Err(StockApiError::InvalidRequest);
        }
        if let Some(location_id) = item.location_id {
            if repository
                .find_active_location_by_id(location_id)
                .await?
                .is_none()
            {
                return Err(StockApiError::LocationNotFound);
            }
        }
        items.push(CreateOutboundOrderItem {
            item_id: item.item_id,
            quantity: validate_positive(item.quantity)?,
            batch_id: item.batch_id,
            location_id: item.location_id,
        });
    }

    let detail = repository
        .create_outbound_order(CreateOutboundOrder {
            destination: normalize_required_text(&request.destination)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
            items,
        })
        .await?;

    outbound_response(detail)
}

/// 分页查询出库单；查询参数在这里统一归一化并转换为仓储查询输入。
pub(crate) async fn list_outbound(
    state: &CoreState,
    query: controller::OutboundListQuery,
) -> Result<PaginatedResponse<controller::OutboundResponse>, StockApiError> {
    if query.item_id.is_some_and(|id| id < 1) {
        return Err(StockApiError::InvalidRequest);
    }
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let status = query
        .status
        .map(|value| normalize_required_text(&value))
        .transpose()?;
    if status
        .as_deref()
        .is_some_and(|value| !matches!(value, "pending" | "approved" | "rejected"))
    {
        return Err(StockApiError::InvalidRequest);
    }
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_outbound_orders(ListOutboundOrders {
            page,
            page_size,
            item_id: query.item_id,
            status,
            date_from: normalize_optional_text(query.date_from)?,
            date_to: normalize_optional_text(query.date_to)?,
            search: normalize_optional_text(query.search)?,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result
            .items
            .into_iter()
            .map(outbound_response)
            .collect::<Result<Vec<_>, StockApiError>>()?,
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询出库历史视角下的筛选值；批次属性只来自指定批次或已审批扣减流水。
pub(crate) async fn outbound_filter_values(
    state: &CoreState,
) -> Result<controller::FilterValuesResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    filter_values_response(repository.list_outbound_filter_values().await?)
}

/// 查询出库单详情；单据不存在时返回 `OutboundOrderNotFound`。
pub(crate) async fn get_outbound(
    state: &CoreState,
    id: i64,
) -> Result<controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_outbound_order_by_id(id).await? else {
        return Err(StockApiError::OutboundOrderNotFound);
    };

    outbound_response(detail)
}

/// 审批出库单；审批事务会按指定批次或 FIFO 扣减库存。
///
/// repository 返回库存不足或非 pending 状态时会映射为稳定库存业务错误。
pub(crate) async fn approve_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .approve_outbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::OutboundOrderNotFound)?;

    outbound_response(detail)
}

/// 拒绝出库单；拒绝不扣减库存，只更新单据状态和审计信息。
pub(crate) async fn reject_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .reject_outbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::OutboundOrderNotFound)?;

    outbound_response(detail)
}
