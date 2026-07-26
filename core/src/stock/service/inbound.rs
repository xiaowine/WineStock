//! 入库单服务。
//!
//! 本模块属于 `stock` 业务服务层，负责入库单创建、分页、筛选值、详情、审批和拒绝。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use crate::{
    persistence::repository::{
        CreateInboundOrder, CreateInboundOrderItem, InboundOrderDetail, ListInboundOrders,
        StockRepository,
    },
    security::CurrentUser,
    state::CoreState,
    stock::{controller, permissions::STOCK_INBOUND_APPROVE_PERMISSION},
};

use super::{
    error::map_stock_db_error,
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{filter_values_response, inbound_response},
    validation::{
        normalize_optional_text, normalize_required_text, validate_non_negative, validate_positive,
    },
    StockApiError,
};

/// 按请求模式创建待审批单据或直接完成入库。
///
/// 本函数会校验物品与库位有效性；直接入库额外要求审核权限，
/// 并在创建事务内同步写入批次、流水和审批审计。
pub(crate) async fn create_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::InboundCreateRequest,
) -> Result<controller::InboundResponse, StockApiError> {
    if request.items.is_empty() || request.items.len() > 256 {
        return Err(StockApiError::InvalidRequest);
    }
    let submission_mode = request.submission_mode;
    let approved_by_user_id = match submission_mode {
        controller::InboundSubmissionMode::PendingApproval => None,
        controller::InboundSubmissionMode::Direct => {
            if !current_user.has_permission(STOCK_INBOUND_APPROVE_PERMISSION) {
                return Err(StockApiError::DirectInboundApprovalForbidden);
            }
            Some(current_user.user_id)
        }
    };
    let repository = StockRepository::new(state.database());
    let mut items = Vec::with_capacity(request.items.len());
    for (line_index, item) in request.items.into_iter().enumerate() {
        if repository
            .find_active_item_by_id(item.item_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::InboundItemInvalid {
                line_index,
                item_id: item.item_id,
            });
        }
        if repository
            .find_active_location_by_id(item.location_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::InboundLocationInvalid {
                line_index,
                location_id: item.location_id,
            });
        }
        items.push(CreateInboundOrderItem {
            item_id: item.item_id,
            quantity: validate_positive(item.quantity)?,
            unit_price: validate_non_negative(Some(item.unit_price))?.expect("输入值已存在"),
            location_id: item.location_id,
            batch_no: normalize_optional_text(item.batch_no)?,
            expires_at: normalize_optional_text(item.expires_at)?,
        });
    }

    let detail = repository
        .create_inbound_order(CreateInboundOrder {
            source: normalize_required_text(&request.source)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
            approved_by_user_id,
            items,
        })
        .await
        .map_err(map_stock_db_error)?;

    let mut response = inbound_response(detail)?;
    response.submission_mode = Some(submission_mode);
    Ok(response)
}

/// 分页查询入库单；查询参数在这里统一归一化并转换为仓储查询输入。
pub(crate) async fn list_inbound(
    state: &CoreState,
    query: controller::InboundListQuery,
) -> Result<PaginatedResponse<controller::InboundResponse>, StockApiError> {
    if query.item_id.is_some_and(|id| id < 1) {
        return Err(StockApiError::InvalidRequest);
    }
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let repository = StockRepository::new(state.database());
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
    let result = repository
        .list_inbound_orders(ListInboundOrders {
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
            .map(inbound_response)
            .collect::<Result<Vec<_>, StockApiError>>()?,
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询入库历史视角下的筛选值；历史值不受当前库存余额影响。
pub(crate) async fn inbound_filter_values(
    state: &CoreState,
) -> Result<controller::FilterValuesResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    filter_values_response(repository.list_inbound_filter_values().await?)
}

/// 查询入库单详情；单据不存在时返回 `InboundOrderNotFound`。
pub(crate) async fn get_inbound(
    state: &CoreState,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_inbound_order_by_id(id).await? else {
        return Err(StockApiError::InboundOrderNotFound);
    };

    inbound_response(detail)
}

/// 审批入库单；审批前重新确认明细物品仍有效，防止等待期间物品被删除。
///
/// 通过校验后由 repository 在事务内写入批次、库存流水和审计事件。
pub(crate) async fn approve_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_inbound_order_by_id(id).await? else {
        return Err(StockApiError::InboundOrderNotFound);
    };
    if detail.order.status != "pending" {
        return Err(StockApiError::OrderNotPending);
    }
    ensure_inbound_items_active(&repository, &detail).await?;

    let detail = repository
        .approve_inbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::InboundOrderNotFound)?;

    inbound_response(detail)
}

/// 拒绝入库单；拒绝不写库存批次或流水，只更新单据状态和审计信息。
pub(crate) async fn reject_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .reject_inbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::InboundOrderNotFound)?;

    inbound_response(detail)
}

/// 在入库审批边界确认每条明细的物品未被软删除；库位有效性由审批事务内部继续把关。
async fn ensure_inbound_items_active(
    repository: &StockRepository<'_>,
    detail: &InboundOrderDetail,
) -> Result<(), StockApiError> {
    for (line_index, item) in detail.items.iter().enumerate() {
        if repository
            .find_active_item_by_id(item.item_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::InboundItemInvalid {
                line_index,
                item_id: item.item_id,
            });
        }
    }
    Ok(())
}
