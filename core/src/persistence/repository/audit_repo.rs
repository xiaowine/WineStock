//! audit 审计事件 repository。
//!
//! 本模块属于 `core` 持久化层，封装跨业务审计事件写入。
//! 业务服务只传递实体、动作和脱敏详情，不直接拼接 `audit_events` 表结构。

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use serde_json::Value;

/// 审计事件写入输入。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordAuditEvent {
    /// 操作用户 ID；系统动作或无法归属用户时允许为空。
    pub user_id: Option<i64>,

    /// 被操作实体类型，例如 `user`。
    pub entity_type: String,

    /// 被操作实体 ID；批量动作或系统动作允许为空。
    pub entity_id: Option<i64>,

    /// 操作动作，必须使用 migration 允许的稳定动作代码。
    pub action: String,

    /// 脱敏后的事件详情 JSON。
    pub details: Option<Value>,
}

/// 审计事件仓储层。
pub(crate) struct AuditRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> AuditRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的审计仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

    /// 写入审计事件；调用方必须保证详情不包含密码、token 或哈希等敏感值。
    pub(crate) async fn record(&self, input: RecordAuditEvent) -> Result<(), DbErr> {
        let details_json = input.details.map(|details| details.to_string());
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO audit_events (user_id, entity_type, entity_id, action, details_json)
                VALUES (?, ?, ?, ?, ?)
                "#,
                [
                    input.user_id.into(),
                    input.entity_type.into(),
                    input.entity_id.into(),
                    input.action.into(),
                    details_json.into(),
                ],
            ))
            .await?;

        Ok(())
    }
}
