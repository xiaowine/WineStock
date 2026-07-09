//! 库存看板聚合查询。
//!
//! 本模块属于 `core` 持久化层，封装库存总览、呆滞料和每日出入库趋势的只读 SQL。
//! 它只返回仓储读取模型，不生成 HTTP 响应结构。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

use super::{
    DailyMovementTrendRecord, DashboardOverviewRecord, SlowMovingStockItemRecord, StockRepository,
};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 查询看板总览；统计只读取当前库存和审批后产生的库存流水。
    pub(crate) async fn dashboard_overview(
        &self,
        slow_moving_days: i64,
    ) -> Result<DashboardOverviewRecord, DbErr> {
        let summary = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT
                    (SELECT COUNT(*) FROM stock_items WHERE deleted_at IS NULL) AS total_items,
                    (
                        SELECT COALESCE(SUM(batches.remaining_quantity), 0.0)
                        FROM stock_batches batches
                        JOIN stock_items items ON items.id = batches.item_id
                        WHERE items.deleted_at IS NULL
                    ) AS total_quantity,
                    (
                        SELECT COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0)
                        FROM stock_batches batches
                        JOIN stock_items items ON items.id = batches.item_id
                        WHERE items.deleted_at IS NULL
                    ) AS total_value,
                    (
                        SELECT COALESCE(SUM(quantity_delta), 0.0)
                        FROM stock_movements
                        WHERE movement_type = 'inbound'
                          AND created_at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days')
                    ) AS inbound_3d,
                    (
                        SELECT COALESCE(SUM(-quantity_delta), 0.0)
                        FROM stock_movements
                        WHERE movement_type = 'outbound'
                          AND created_at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days')
                    ) AS outbound_3d
                "#,
                [],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("dashboard overview".to_owned()))?;
        let slow_moving_items = self.list_slow_moving_items(slow_moving_days).await?;

        Ok(DashboardOverviewRecord {
            total_items: summary.try_get("", "total_items")?,
            total_quantity: summary.try_get("", "total_quantity")?,
            total_value: summary.try_get("", "total_value")?,
            inbound_3d: summary.try_get("", "inbound_3d")?,
            outbound_3d: summary.try_get("", "outbound_3d")?,
            slow_moving_items,
        })
    }

    /// 查询每日出入库趋势；无流水日期也会返回 0，便于前端直接绘图。
    pub(crate) async fn dashboard_trends(
        &self,
        days: i64,
    ) -> Result<Vec<DailyMovementTrendRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH RECURSIVE dates(date, remaining) AS (
                    SELECT date('now', ?), ?
                    UNION ALL
                    SELECT date(date, '+1 day'), remaining - 1
                    FROM dates
                    WHERE remaining > 0
                ),
                movement_daily AS (
                    SELECT
                        date(created_at) AS date,
                        SUM(CASE WHEN movement_type = 'inbound' THEN quantity_delta ELSE 0.0 END) AS inbound_quantity,
                        SUM(CASE WHEN movement_type = 'outbound' THEN -quantity_delta ELSE 0.0 END) AS outbound_quantity
                    FROM stock_movements
                    WHERE movement_type IN ('inbound', 'outbound')
                      AND date(created_at) >= date('now', ?)
                    GROUP BY date(created_at)
                )
                SELECT
                    dates.date AS date,
                    COALESCE(movement_daily.inbound_quantity, 0.0) AS inbound_quantity,
                    COALESCE(movement_daily.outbound_quantity, 0.0) AS outbound_quantity
                FROM dates
                LEFT JOIN movement_daily ON movement_daily.date = dates.date
                ORDER BY dates.date ASC
                "#,
                vec![
                    format!("-{} days", days - 1).into(),
                    (days - 1).into(),
                    format!("-{} days", days - 1).into(),
                ],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(DailyMovementTrendRecord {
                    date: row.try_get("", "date")?,
                    inbound_quantity: row.try_get("", "inbound_quantity")?,
                    outbound_quantity: row.try_get("", "outbound_quantity")?,
                })
            })
            .collect()
    }

    async fn list_slow_moving_items(
        &self,
        slow_moving_days: i64,
    ) -> Result<Vec<SlowMovingStockItemRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                WITH item_stock AS (
                    SELECT
                        items.id AS item_id,
                        items.name AS item_name,
                        COALESCE(SUM(batches.remaining_quantity), 0.0) AS quantity,
                        COALESCE(SUM(batches.remaining_quantity * batches.unit_cost), 0.0) AS value,
                        MAX(movements.created_at) AS last_movement_at
                    FROM stock_items items
                    LEFT JOIN stock_batches batches ON batches.item_id = items.id
                    LEFT JOIN stock_movements movements ON movements.item_id = items.id
                    WHERE items.deleted_at IS NULL
                    GROUP BY items.id, items.name
                )
                SELECT
                    item_id,
                    item_name,
                    quantity,
                    value,
                    CAST(COALESCE(julianday('now') - julianday(last_movement_at), ? + 1) AS INTEGER)
                        AS days_since_last_movement
                FROM item_stock
                WHERE quantity > 0
                  AND COALESCE(julianday('now') - julianday(last_movement_at), ? + 1) >= ?
                ORDER BY days_since_last_movement DESC, item_id ASC
                "#,
                vec![
                    slow_moving_days.into(),
                    slow_moving_days.into(),
                    slow_moving_days.into(),
                ],
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(SlowMovingStockItemRecord {
                    item_id: row.try_get("", "item_id")?,
                    item_name: row.try_get("", "item_name")?,
                    quantity: row.try_get("", "quantity")?,
                    value: row.try_get("", "value")?,
                    days_since_last_movement: row.try_get("", "days_since_last_movement")?,
                })
            })
            .collect()
    }
}
