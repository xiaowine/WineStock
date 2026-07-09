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
        "auth_permissions",
        "auth_user_permission_assignments",
        "auth_settings",
        "auth_signing_keys",
        "auth_refresh_tokens",
        "storage_file_objects",
        "stock_templates",
        "stock_template_fields",
        "stock_items",
        "stock_inbound_orders",
        "stock_inbound_order_items",
        "stock_outbound_orders",
        "stock_outbound_order_items",
        "stock_batches",
        "stock_movements",
        "stock_substitutes",
        "audit_events",
    ] {
        let sql = format!(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = '{table}'"
        );
        assert_eq!(query_i64(&storage.database, &sql, "count").await, 1);
    }
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_users') WHERE name = 'password_change_required'",
            "count",
        )
        .await,
        1
    );
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
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_stock_items_sku_active'",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_stock_templates_name_active'",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_audit_events_entity_created'",
            "count",
        )
        .await,
        1
    );

    storage
        .database
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO stock_templates (name, description) VALUES ('URL Template', NULL)"
                .to_owned(),
        ))
        .await
        .expect("template should insert");
    storage
        .database
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO stock_template_fields (template_id, field_name, field_type) VALUES (1, 'datasheet', 'url')"
                .to_owned(),
        ))
        .await
        .expect("url field type should pass schema check");

    storage
        .database
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO auth_users (username, password_hash) VALUES ('web-user', 'hash')"
                .to_owned(),
        ))
        .await
        .expect("web user should insert");
    storage
        .database
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO auth_refresh_tokens (
                user_id,
                token_hash,
                device_name,
                client_kind,
                app_version,
                refresh_token_version,
                expires_at
            )
            VALUES (
                1,
                'web-token-hash',
                'browser',
                'web',
                '0.1.0-web',
                'v1',
                '2099-01-01T00:00:00.000Z'
            )
            "#
            .to_owned(),
        ))
        .await
        .expect("web client kind should pass schema check");
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
