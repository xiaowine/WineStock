//! 库存事件日志服务。
//!
//! 本模块属于 `stock` 业务服务层，负责审计事件分页查询和筛选条件归一化。
//! 它不写审计事件，也不处理 HTTP 路由或权限中间件。

use crate::{
    persistence::repository::{ListAuditEvents, StockRepository},
    state::CoreState,
    stock::controller,
};

use super::{
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::event_response,
    validation::{normalize_optional_text, positive_id},
    StockApiError,
};

/// 分页查询审计事件日志；筛选条件在这里完成文本裁剪和正 ID 校验。
pub(crate) async fn list_events(
    state: &CoreState,
    query: controller::EventListQuery,
) -> Result<PaginatedResponse<controller::EventLogResponse>, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_audit_events(ListAuditEvents {
            page,
            page_size,
            entity_type: normalize_optional_text(query.entity_type)?,
            entity_id: query.entity_id.map(positive_id).transpose()?,
            action: normalize_optional_text(query.action)?,
            user_id: query.user_id.map(positive_id).transpose()?,
            date_from: normalize_optional_text(query.date_from)?,
            date_to: normalize_optional_text(query.date_to)?,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result.items.into_iter().map(event_response).collect(),
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}
