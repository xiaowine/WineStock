//! SQLite 连接和 migration 测试。

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tempfile::tempdir;
use winestock_shared::StorageConfig;

use super::*;

#[tokio::test]
async fn opens_sqlite_database_with_wal_pragmas() {
    let temp = tempdir().expect("temp dir should exist");
    let config = StorageConfig {
        database_path: temp
            .path()
            .join("winestock.sqlite")
            .to_string_lossy()
            .into_owned(),
        files_dir: temp.path().join("files").to_string_lossy().into_owned(),
        auto_migrate: true,
    };

    let storage = open_sqlite_storage(&config)
        .await
        .expect("storage should open");

    assert!(storage.database_path.exists());
    assert_eq!(
        query_string(&storage.database, "PRAGMA journal_mode", "journal_mode").await,
        "wal"
    );
    assert_eq!(
        query_i64(&storage.database, "PRAGMA foreign_keys", "foreign_keys").await,
        1
    );
}

#[tokio::test]
async fn migration_is_idempotent_and_creates_v1_tables() {
    let temp = tempdir().expect("temp dir should exist");
    let config = StorageConfig {
        database_path: temp
            .path()
            .join("winestock.sqlite")
            .to_string_lossy()
            .into_owned(),
        files_dir: temp.path().join("files").to_string_lossy().into_owned(),
        auto_migrate: true,
    };
    let storage = open_sqlite_storage(&config)
        .await
        .expect("storage should open");

    migrate_storage_schema(&storage)
        .await
        .expect("first migration should succeed");
    migrate_storage_schema(&storage)
        .await
        .expect("second migration should be idempotent");

    for table in [
        "auth_users",
        "auth_roles",
        "auth_user_role_assignments",
        "auth_permissions",
        "auth_role_permission_assignments",
        "auth_settings",
        "auth_signing_keys",
        "auth_refresh_tokens",
        "storage_file_objects",
    ] {
        let sql = format!(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = '{table}'"
        );
        assert_eq!(query_i64(&storage.database, &sql, "count").await, 1);
    }
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_refresh_tokens') WHERE name = 'session_id'",
            "count",
        )
        .await,
        0
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_refresh_tokens') WHERE name = 'app_version'",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_refresh_tokens') WHERE name = 'refresh_token_version'",
            "count",
        )
        .await,
        1
    );
}

async fn query_string(database: &DatabaseConnection, sql: &str, column: &str) -> String {
    database
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_owned(),
        ))
        .await
        .expect("query should execute")
        .expect("row should exist")
        .try_get("", column)
        .expect("column should decode")
}

async fn query_i64(database: &DatabaseConnection, sql: &str, column: &str) -> i64 {
    database
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_owned(),
        ))
        .await
        .expect("query should execute")
        .expect("row should exist")
        .try_get("", column)
        .expect("column should decode")
}
