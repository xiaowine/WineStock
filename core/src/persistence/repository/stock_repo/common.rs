//! stock repository 共用数据库辅助函数。
//!
//! 本模块属于 `core` 持久化层，只放多个库存仓储子模块共享的事务辅助逻辑。
//! 具体业务表查询应留在对应子模块。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

/// 查询指定物品当前所有批次余额；审批流程用它写入库存流水后的账面余额。
pub(super) async fn current_item_quantity_on_connection<C>(
    connection: &C,
    item_id: i64,
) -> Result<f64, DbErr>
where
    C: ConnectionTrait,
{
    let row = connection
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT COALESCE(SUM(remaining_quantity), 0.0) AS quantity
            FROM stock_batches
            WHERE item_id = ?
            "#,
            [item_id.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("stock batch balance".to_owned()))?;

    row.try_get("", "quantity")
}

/// 在调用方事务内写入审计事件；调用方必须传入已脱敏的详情 JSON。
pub(super) async fn insert_audit_event_on_connection<C>(
    connection: &C,
    user_id: Option<i64>,
    entity_type: &str,
    entity_id: Option<i64>,
    action: &str,
    details_json: Option<String>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO audit_events (user_id, entity_type, entity_id, action, details_json)
            VALUES (?, ?, ?, ?, ?)
            "#,
            vec![
                user_id.into(),
                entity_type.into(),
                entity_id.into(),
                action.into(),
                details_json.into(),
            ],
        ))
        .await?;

    Ok(())
}

/// 把审计详情中的动态文本安全编码为 JSON 字符串字面量。
pub(super) fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}
