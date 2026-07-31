//! 库存审计事件查询。
//!
//! 本模块属于 `core` 持久化层，封装库存业务使用的审计事件分页和筛选查询。
//! 审计事件写入复用 `common` 中的事务辅助函数。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, Value};

use super::{AuditEventRecord, ListAuditEvents, Page, StockRepository};

impl<'db, C> StockRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 分页查询审计事件，支持实体、动作、用户和时间筛选。
    pub(crate) async fn list_audit_events(
        &self,
        input: ListAuditEvents,
    ) -> Result<Page<AuditEventRecord>, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let total = self.count_audit_events(&input).await?;
        let items = self.query_audit_events(&input, limit, offset).await?;

        Ok(Page { items, total })
    }

    async fn count_audit_events(&self, input: &ListAuditEvents) -> Result<u64, DbErr> {
        let (where_clause, values) = audit_event_filters(input);
        let row = self
            .database
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS count FROM audit_events {where_clause}"),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("audit event count".to_owned()))?;
        let count: i64 = row.try_get("", "count")?;

        Ok(count as u64)
    }

    async fn query_audit_events(
        &self,
        input: &ListAuditEvents,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditEventRecord>, DbErr> {
        let (where_clause, mut values) = audit_event_filters(input);
        values.push(limit.into());
        values.push(offset.into());
        let rows = self
            .database
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    r#"
                    SELECT
                        audit_events.id,
                        audit_events.timestamp,
                        audit_events.user_id,
                        auth_users.username,
                        audit_events.entity_type,
                        audit_events.entity_id,
                        audit_events.action,
                        audit_events.details_json
                    FROM audit_events
                    LEFT JOIN auth_users ON auth_users.id = audit_events.user_id
                    {where_clause}
                    ORDER BY audit_events.timestamp DESC, audit_events.id DESC
                    LIMIT ? OFFSET ?
                    "#
                ),
                values,
            ))
            .await?;

        rows.into_iter().map(audit_event_from_row).collect()
    }
}

fn audit_event_filters(input: &ListAuditEvents) -> (String, Vec<Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(entity_type) = input.entity_type.as_ref() {
        clauses.push("entity_type = ?");
        values.push(entity_type.clone().into());
    }
    if let Some(entity_id) = input.entity_id {
        clauses.push("entity_id = ?");
        values.push(entity_id.into());
    }
    if let Some(action) = input.action.as_ref() {
        clauses.push("action = ?");
        values.push(action.clone().into());
    }
    if let Some(user_id) = input.user_id {
        clauses.push("user_id = ?");
        values.push(user_id.into());
    }
    if let Some(date_from) = input.date_from.as_ref() {
        clauses.push("timestamp >= ?");
        values.push(date_from.clone().into());
    }
    if let Some(date_to) = input.date_to.as_ref() {
        clauses.push("timestamp <= ?");
        values.push(date_to.clone().into());
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

fn audit_event_from_row(row: sea_orm::QueryResult) -> Result<AuditEventRecord, DbErr> {
    Ok(AuditEventRecord {
        id: row.try_get("", "id")?,
        timestamp: row.try_get("", "timestamp")?,
        user_id: row.try_get("", "user_id")?,
        username: row.try_get("", "username")?,
        entity_type: row.try_get("", "entity_type")?,
        entity_id: row.try_get("", "entity_id")?,
        action: row.try_get("", "action")?,
        details_json: row.try_get("", "details_json")?,
    })
}
