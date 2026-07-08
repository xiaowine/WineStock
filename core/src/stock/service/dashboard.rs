//! 库存看板服务。
//!
//! 本模块属于 `stock` 业务服务层，负责库存总览和出入库趋势只读查询。
//! 它不处理 HTTP 路由、权限中间件或平台展示逻辑。

use crate::{persistence::repository::StockRepository, state::CoreState, stock::controller};

use super::{response::dashboard_overview_response, StockApiError};

/// 看板趋势默认天数。
const DEFAULT_TREND_DAYS: u64 = 30;

/// 看板趋势最大天数，避免单次返回过多图表点。
const MAX_TREND_DAYS: u64 = 365;

/// 呆滞料阈值天数；首版接口固定为 30 天。
const SLOW_MOVING_DAYS: i64 = 30;

/// 查询库存看板总览；只读取当前库存和审批后库存流水。
pub(crate) async fn dashboard_overview(
    state: &CoreState,
) -> Result<controller::DashboardOverviewResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let overview = repository.dashboard_overview(SLOW_MOVING_DAYS).await?;

    Ok(dashboard_overview_response(overview))
}

/// 查询出入库趋势；查询天数会归一到 1 到 365 天。
pub(crate) async fn dashboard_trends(
    state: &CoreState,
    query: controller::TrendsQuery,
) -> Result<controller::TrendsResponse, StockApiError> {
    let days = query
        .days
        .unwrap_or(DEFAULT_TREND_DAYS)
        .clamp(1, MAX_TREND_DAYS) as i64;
    let repository = StockRepository::new(state.database());
    let daily = repository
        .dashboard_trends(days)
        .await?
        .into_iter()
        .map(|record| controller::DailyTrend {
            date: record.date,
            inbound_quantity: record.inbound_quantity,
            outbound_quantity: record.outbound_quantity,
        })
        .collect();

    Ok(controller::TrendsResponse { daily })
}
