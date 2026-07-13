//! 为物品属性定义增加目录展示标记。
//!
//! 本 migration 属于 core 持久化层，负责让已经执行过初始 schema 的数据库获得
//! `catalog_visible` 列；新数据库的初始 schema 已包含该列，因此这里会先检查再变更。

use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DatabaseBackend, Statement},
};

/// 增加物品目录展示属性标记。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// 已有数据库缺少字段时增加字段；新数据库直接跳过。
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !column_exists(manager, "catalog_visible").await? {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE stock_item_attribute_definitions ADD COLUMN catalog_visible INTEGER NOT NULL DEFAULT 0 CHECK (catalog_visible IN (0, 1))",
                )
                .await?;
        }
        Ok(())
    }

    /// 回滚只在字段存在时删除字段，保持重复执行的确定性。
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if column_exists(manager, "catalog_visible").await? {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE stock_item_attribute_definitions DROP COLUMN catalog_visible",
                )
                .await?;
        }
        Ok(())
    }
}

/// 查询 SQLite 表结构，避免新数据库在初始 schema 已含字段时重复 ALTER。
async fn column_exists(manager: &SchemaManager<'_>, column: &str) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA table_info(stock_item_attribute_definitions)".to_owned(),
        ))
        .await?;
    Ok(rows.into_iter().any(|row| {
        row.try_get::<String>("", "name")
            .is_ok_and(|name| name == column)
    }))
}
