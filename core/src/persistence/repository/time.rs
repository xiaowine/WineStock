//! SQLite 时间工具。
//!
//! 本模块属于 core 持久化层，集中生成与 SQLite 表默认值一致的 UTC 时间字符串。
//! 它不拥有任何具体业务 repository，只提供仓储层复用的时间查询。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

/// 从 SQLite 读取统一时间戳，避免 Rust 进程时间和数据库默认时间格式不一致。
pub(crate) async fn sqlite_now(database: &impl ConnectionTrait) -> Result<String, DbErr> {
    let row = database
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now') AS current_time".to_owned(),
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("SQLite current timestamp".to_owned()))?;

    row.try_get("", "current_time")
}

/// 从 SQLite 生成当前时间之后若干秒的 UTC 时间戳，保持与表默认时间格式一致。
pub(crate) async fn sqlite_time_after_seconds(
    database: &impl ConnectionTrait,
    seconds: u64,
) -> Result<String, DbErr> {
    let row = database
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+{seconds} seconds') AS target_time"
            ),
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("SQLite target timestamp".to_owned()))?;

    row.try_get("", "target_time")
}
