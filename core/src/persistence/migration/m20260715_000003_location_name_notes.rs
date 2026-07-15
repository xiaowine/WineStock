//! 删除冗余库位编码并增加库位备注。
//!
//! 本 migration 属于 core 持久化层，先建立库位名称唯一约束，再移除旧编码字段，
//! 避免已有数据库在名称重复时静默丢失用于区分库位的信息。

use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DatabaseBackend, Statement},
};

/// 将库位自然标识统一为全局唯一名称，并增加可选备注。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_locations_name_active ON stock_locations(name) WHERE deleted_at IS NULL",
            )
            .await?;
        if !column_exists(manager, "notes").await? {
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE stock_locations ADD COLUMN notes TEXT")
                .await?;
        }
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_stock_locations_code_active")
            .await?;
        if column_exists(manager, "code").await? {
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE stock_locations DROP COLUMN code")
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !column_exists(manager, "code").await? {
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE stock_locations ADD COLUMN code TEXT")
                .await?;
            manager
                .get_connection()
                .execute_unprepared("UPDATE stock_locations SET code = name")
                .await?;
        }
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_locations_code_active ON stock_locations(code) WHERE deleted_at IS NULL",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_stock_locations_name_active")
            .await?;
        if column_exists(manager, "notes").await? {
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE stock_locations DROP COLUMN notes")
                .await?;
        }
        Ok(())
    }
}

async fn column_exists(manager: &SchemaManager<'_>, column: &str) -> Result<bool, DbErr> {
    let rows = manager
        .get_connection()
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA table_info(stock_locations)".to_owned(),
        ))
        .await?;
    Ok(rows.into_iter().any(|row| {
        row.try_get::<String>("", "name")
            .is_ok_and(|name| name == column)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::sea_orm::Database;

    #[tokio::test]
    async fn migration_preserves_locations_and_adds_notes() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("测试数据库应可连接");
        database
            .execute_unprepared(
                r#"
                CREATE TABLE stock_locations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    group_id INTEGER NOT NULL,
                    code TEXT NOT NULL,
                    name TEXT NOT NULL,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                CREATE UNIQUE INDEX idx_stock_locations_code_active
                    ON stock_locations(code) WHERE deleted_at IS NULL;
                INSERT INTO stock_locations
                    (group_id, code, name, sort_order, created_at, updated_at)
                VALUES (1, 'A-01', '入口库位', 0, 'now', 'now');
                "#,
            )
            .await
            .expect("旧库位结构应可建立");

        Migration
            .up(&SchemaManager::new(&database))
            .await
            .expect("库位迁移应成功");

        let columns = database
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA table_info(stock_locations)".to_owned(),
            ))
            .await
            .expect("应可读取迁移后结构")
            .into_iter()
            .map(|row| row.try_get::<String>("", "name").expect("列名应可读取"))
            .collect::<Vec<_>>();
        assert!(!columns.iter().any(|column| column == "code"));
        assert!(columns.iter().any(|column| column == "notes"));

        let row = database
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT name, notes FROM stock_locations WHERE id = 1".to_owned(),
            ))
            .await
            .expect("应可查询迁移后库位")
            .expect("原库位应保留");
        assert_eq!(row.try_get::<String>("", "name").unwrap(), "入口库位");
        assert_eq!(row.try_get::<Option<String>>("", "notes").unwrap(), None);
    }

    #[tokio::test]
    async fn migration_rejects_duplicate_active_names_before_dropping_code() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("测试数据库应可连接");
        database
            .execute_unprepared(
                r#"
                CREATE TABLE stock_locations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    group_id INTEGER NOT NULL,
                    code TEXT NOT NULL,
                    name TEXT NOT NULL,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                INSERT INTO stock_locations
                    (group_id, code, name, sort_order, created_at, updated_at)
                VALUES
                    (1, 'A-01', '重复库位', 0, 'now', 'now'),
                    (2, 'B-01', '重复库位', 0, 'now', 'now');
                "#,
            )
            .await
            .expect("旧库位数据应可建立");

        assert!(Migration.up(&SchemaManager::new(&database)).await.is_err());
        let columns = database
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA table_info(stock_locations)".to_owned(),
            ))
            .await
            .expect("应可读取失败后的结构")
            .into_iter()
            .map(|row| row.try_get::<String>("", "name").expect("列名应可读取"))
            .collect::<Vec<_>>();
        assert!(columns.iter().any(|column| column == "code"));
        assert!(!columns.iter().any(|column| column == "notes"));
    }
}
