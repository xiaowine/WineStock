//! SQLite 连接和 migration 测试。

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
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

    let sqlite_version = query_string(
        &storage.database,
        "SELECT sqlite_version() AS version",
        "version",
    )
    .await;
    assert!(
        sqlite_version_at_least(&sqlite_version, 3, 35),
        "SQLite {sqlite_version} does not support RETURNING; version 3.35 or newer is required"
    );
}

#[tokio::test]
async fn sqlite_returning_row_does_not_escape_a_rolled_back_transaction() {
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
        .expect("schema should migrate");
    let transaction = storage
        .database
        .begin()
        .await
        .expect("transaction should begin");

    let returned = transaction
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO stock_location_groups
                (parent_id, name, sort_order, created_at, updated_at)
            VALUES (NULL, 'RETURNING 回滚测试', 0, '2026-07-29T00:00:00.000Z', '2026-07-29T00:00:00.000Z')
            RETURNING id, name
            "#
            .to_owned(),
        ))
        .await
        .expect("returning insert should execute")
        .expect("returning insert should return one row");
    let returned_id: i64 = returned.try_get("", "id").expect("id should decode");
    assert!(returned_id > 0);
    assert_eq!(
        returned
            .try_get::<String>("", "name")
            .expect("name should decode"),
        "RETURNING 回滚测试"
    );

    let conflict = transaction
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO stock_location_groups
                (parent_id, name, sort_order, created_at, updated_at)
            VALUES (NULL, 'RETURNING 回滚测试', 0, '2026-07-29T00:00:00.000Z', '2026-07-29T00:00:00.000Z')
            RETURNING id
            "#
            .to_owned(),
        ))
        .await;
    assert!(conflict.is_err(), "duplicate group should fail");
    transaction
        .rollback()
        .await
        .expect("failed workflow should roll back");

    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM stock_location_groups WHERE name = 'RETURNING 回滚测试'",
            "count",
        )
        .await,
        0
    );
}

#[tokio::test]
async fn migration_is_idempotent_and_creates_current_schema() {
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

    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM seaql_migrations",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_string(
            &storage.database,
            "SELECT version FROM seaql_migrations LIMIT 1",
            "version",
        )
        .await,
        "m20260706_000001_initial_schema"
    );

    for table in [
        "auth_users",
        "auth_permissions",
        "auth_user_permission_assignments",
        "auth_settings",
        "auth_signing_keys",
        "auth_refresh_tokens",
        "storage_file_objects",
        "stock_item_categories",
        "stock_item_attribute_templates",
        "stock_item_attribute_definitions",
        "stock_items",
        "stock_item_attributes",
        "storage_item_file_bindings",
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
    for column in [
        "template_id",
        "owner_item_id",
        "unit_mode",
        "fixed_unit",
        "unit_options_json",
        "catalog_visible",
    ] {
        let sql = format!(
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_item_attribute_definitions') WHERE name = '{column}'"
        );
        assert_eq!(query_i64(&storage.database, &sql, "count").await, 1);
    }
    for removed_table in [
        "stock_item_attribute_template_fields",
        "stock_inbound_templates",
        "stock_inbound_template_fields",
        "stock_inbound_order_item_attributes",
        "storage_inbound_file_bindings",
    ] {
        let sql = format!(
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = '{removed_table}'"
        );
        assert_eq!(query_i64(&storage.database, &sql, "count").await, 0);
    }
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_item_attributes') WHERE name IN ('template_field_id', 'field_name', 'field_type')",
            "count",
        )
        .await,
        0
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_item_attributes') WHERE name = 'definition_id' AND [notnull] = 1",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_inbound_order_items') WHERE name = 'inbound_template_id'",
            "count",
        )
        .await,
        0
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_item_attribute_templates') WHERE name = 'default_inbound_template_id'",
            "count",
        )
        .await,
        0
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_users') WHERE name = 'display_name'",
            "count",
        )
        .await,
        0
    );
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
            "SELECT COUNT(*) AS count FROM pragma_table_info('auth_users') WHERE name = 'deleted_at'",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_locations') WHERE name IN ('notes')",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM pragma_table_info('stock_locations') WHERE name = 'code'",
            "count",
        )
        .await,
        0
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_auth_users_visible_id'",
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
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_stock_locations_name_active'",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &storage.database,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_stock_item_attribute_templates_name_active'",
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
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO stock_item_attribute_templates (name, description) VALUES ('URL Template', NULL)"
                .to_owned(),
        ))
        .await
        .expect("template should insert");
    storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO stock_item_attribute_definitions (template_id, field_name, field_type) VALUES (1, 'datasheet', 'url')"
                .to_owned(),
        ))
        .await
        .expect("url field type should pass schema check");

    storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO auth_users (username, password_hash) VALUES ('web-user', 'hash')"
                .to_owned(),
        ))
        .await
        .expect("web user should insert");
    storage
        .database
        .execute_raw(Statement::from_string(
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
        .query_one_raw(Statement::from_string(
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
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_owned(),
        ))
        .await
        .expect("query should execute")
        .expect("row should exist")
        .try_get("", column)
        .expect("column should decode")
}

fn sqlite_version_at_least(version: &str, required_major: u32, required_minor: u32) -> bool {
    let mut parts = version.split('.');
    let Some(major) = parts.next().and_then(|part| part.parse::<u32>().ok()) else {
        return false;
    };
    let Some(minor) = parts.next().and_then(|part| part.parse::<u32>().ok()) else {
        return false;
    };

    (major, minor) >= (required_major, required_minor)
}
