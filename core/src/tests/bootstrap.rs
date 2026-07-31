//! core 启动流程测试。

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tempfile::tempdir;
use winestock_shared::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};

use super::*;

#[tokio::test]
async fn self_hosted_bootstrap_initializes_auth_defaults_and_key() {
    let temp = tempdir().expect("temp dir should exist");
    let config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let first = bootstrap_from_config(&config)
        .await
        .expect("bootstrap should succeed")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(first.auth.settings.access_token_ttl_seconds, 900);
    assert_eq!(first.auth.settings.refresh_token_ttl_seconds, 604800);
    assert_eq!(first.auth.active_signing_key.algorithm, "HS256");
    assert_eq!(
        first.auth.active_signing_key.status,
        crate::SigningKeyStatus::Active
    );
    assert!(first.auth.active_signing_key.key_id.starts_with("ak_"));
    assert!(!first.auth.active_signing_key.key_material.is_empty());
    assert!(first.auth.admin_setup_required);

    let second = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should succeed")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(
        first.auth.active_signing_key.key_id,
        second.auth.active_signing_key.key_id
    );
    assert_eq!(
        first.auth.active_signing_key.key_material,
        second.auth.active_signing_key.key_material
    );

    let active_count: i64 = query_i64(
        &second.storage.database,
        "SELECT COUNT(*) AS count FROM auth_signing_keys WHERE status = 'active'",
        "count",
    )
    .await;
    assert_eq!(active_count, 1);

    let user_count = query_i64(
        &second.storage.database,
        "SELECT COUNT(*) AS count FROM auth_users",
        "count",
    )
    .await;
    assert_eq!(user_count, 0);

    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT key FROM auth_settings ORDER BY key",
            "key",
        )
        .await,
        vec!["access_token_ttl_seconds", "refresh_token_ttl_seconds"]
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT code FROM auth_permissions ORDER BY code",
            "code",
        )
        .await,
        vec![
            "audit.read",
            "stock.dashboard.read",
            "stock.inbound.approve",
            "stock.inbound.create",
            "stock.inbound.read",
            "stock.item.manage",
            "stock.item.read",
            "stock.location.manage",
            "stock.location.read",
            "stock.outbound.approve",
            "stock.outbound.create",
            "stock.outbound.read",
            "stock.read",
            "stock.substitute.manage",
            "stock.substitute.read",
            "stock.template.manage",
            "stock.template.read",
            "stock.write",
            "user.delete",
            "user.password.reset",
            "user.permission.read",
            "user.permissions.update",
            "user.read",
            "user.register",
            "user.status.update",
        ]
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM auth_user_permission_assignments",
            "count",
        )
        .await,
        0
    );

    let mut template_names = query_string_vec(
        &second.storage.database,
        "SELECT name FROM stock_item_attribute_templates WHERE deleted_at IS NULL",
        "name",
    )
    .await;
    template_names.sort();
    assert_eq!(
        template_names,
        vec![
            "3D打印耗材属性".to_owned(),
            "元器件属性".to_owned(),
            "通用物品属性".to_owned()
        ]
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM stock_item_attribute_definitions",
            "count",
        )
        .await,
        14
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM stock_item_attribute_definitions WHERE field_type = 'url'",
            "count",
        )
        .await,
        2
    );
    assert_eq!(
        item_template_field_names(&second.storage.database, "元器件属性").await,
        vec!["型号", "品牌", "封装", "参数", "数据手册", "产品图片"]
    );
    assert_eq!(
        item_template_field_names(&second.storage.database, "3D打印耗材属性").await,
        vec!["材质", "颜色", "线径", "品牌", "产品链接"]
    );
    assert_eq!(
        item_template_field_names(&second.storage.database, "通用物品属性").await,
        vec!["品牌", "规格型号", "用途"]
    );
}

#[tokio::test]
async fn client_only_bootstrap_does_not_touch_storage() {
    let temp = tempdir().expect("temp dir should exist");
    let missing_database = temp.path().join("missing").join("winestock.sqlite");
    let config = test_config(
        RuntimeMode::ClientOnly,
        missing_database.to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let bootstrap = bootstrap_from_config(&config)
        .await
        .expect("client-only should skip storage");

    assert!(!bootstrap.initialized_local_service());
    assert!(!missing_database.exists());
}

#[tokio::test]
async fn local_bootstrap_initializes_rbac_before_auth_runtime() {
    let temp = tempdir().expect("temp dir should exist");
    let mut config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );
    config.storage.auto_migrate = false;

    let error = bootstrap_from_config(&config)
        .await
        .expect_err("missing schema should fail before auth runtime initializes");

    assert!(matches!(error, CoreBootstrapError::Rbac(_)));
}

#[tokio::test]
async fn server_mode_bootstrap_uses_local_storage() {
    let temp = tempdir().expect("temp dir should exist");
    let database = temp.path().join("server.sqlite");
    let config = test_config(
        RuntimeMode::ServerMode,
        database.to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let bootstrap = bootstrap_from_config(&config)
        .await
        .expect("server-mode should use storage");

    assert!(bootstrap.initialized_local_service());
    assert!(database.exists());
}

#[tokio::test]
async fn auth_defaults_do_not_overwrite_database_managed_settings() {
    let temp = tempdir().expect("temp dir should exist");
    let config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let first = bootstrap_from_config(&config)
        .await
        .expect("first bootstrap should initialize settings")
        .local_service
        .expect("local service should be initialized");
    first
        .storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE auth_settings SET value = '1200' WHERE key = 'access_token_ttl_seconds'"
                .to_owned(),
        ))
        .await
        .expect("setting should update");

    let bootstrap = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should preserve settings")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(bootstrap.auth.settings.access_token_ttl_seconds, 1200);
}

#[tokio::test]
async fn builtin_rbac_bootstrap_is_idempotent_and_preserves_existing_permission_text() {
    let temp = tempdir().expect("temp dir should exist");
    let config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let first = bootstrap_from_config(&config)
        .await
        .expect("first bootstrap should initialize rbac")
        .local_service
        .expect("local service should be initialized");
    first
        .storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE auth_permissions SET description = '自定义用户读取说明' WHERE code = 'user.read'"
                .to_owned(),
        ))
        .await
        .expect("permission should update");

    let second = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should preserve rbac")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM auth_permissions",
            "count",
        )
        .await,
        25
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT description FROM auth_permissions WHERE code = 'user.read'",
            "description",
        )
        .await,
        vec!["自定义用户读取说明"]
    );
}

#[tokio::test]
async fn default_attribute_templates_are_idempotent_and_preserve_user_changes() {
    let temp = tempdir().expect("temp dir should exist");
    let config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let first = bootstrap_from_config(&config)
        .await
        .expect("first bootstrap should initialize stock defaults")
        .local_service
        .expect("local service should be initialized");
    first
        .storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE stock_item_attribute_templates SET description = '自定义元器件模板' WHERE name = '元器件属性'"
                .to_owned(),
        ))
        .await
        .expect("template should update");
    first
        .storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE stock_item_attribute_templates SET deleted_at = '2026-07-09T00:00:00.000Z' WHERE name = '通用物品属性'"
                .to_owned(),
        ))
        .await
        .expect("template should soft delete");

    let second = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should preserve stock defaults")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM stock_item_attribute_templates WHERE name IN ('元器件属性', '3D打印耗材属性', '通用物品属性')",
            "count",
        )
        .await,
        3
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT description FROM stock_item_attribute_templates WHERE name = '元器件属性'",
            "description",
        )
        .await,
        vec!["自定义元器件模板"]
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM stock_item_attribute_templates WHERE name = '通用物品属性' AND deleted_at IS NULL",
            "count",
        )
        .await,
        0
    );
}

#[tokio::test]
async fn default_stock_location_reuses_existing_group_when_location_was_removed() {
    let temp = tempdir().expect("temp dir should exist");
    let config = test_config(
        RuntimeMode::SelfHosted,
        temp.path().join("winestock.sqlite").to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let first = bootstrap_from_config(&config)
        .await
        .expect("first bootstrap should initialize stock defaults")
        .local_service
        .expect("local service should be initialized");
    let default_group_id = query_i64(
        &first.storage.database,
        "SELECT id FROM stock_location_groups WHERE parent_id IS NULL AND name = '示例库区' AND deleted_at IS NULL",
        "id",
    )
    .await;
    first
        .storage
        .database
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE stock_locations SET deleted_at = '2026-07-09T00:00:00.000Z' WHERE name = '示例库位'"
                .to_owned(),
        ))
        .await
        .expect("default location should soft delete");

    let second = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should recreate default location")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM stock_location_groups WHERE parent_id IS NULL AND name = '示例库区' AND deleted_at IS NULL",
            "count",
        )
        .await,
        1
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT group_id FROM stock_locations WHERE name = '示例库位' AND deleted_at IS NULL",
            "group_id",
        )
        .await,
        default_group_id
    );
}

#[tokio::test]
async fn self_hosted_bootstrap_requires_database_directory() {
    let temp = tempdir().expect("temp dir should exist");
    let missing_database = temp.path().join("missing").join("winestock.sqlite");
    let config = test_config(
        RuntimeMode::SelfHosted,
        missing_database.to_string_lossy(),
        temp.path().join("files").to_string_lossy(),
    );

    let error = bootstrap_from_config(&config)
        .await
        .expect_err("missing directory should fail");

    assert!(matches!(
        error,
        CoreBootstrapError::Storage(StorageBootstrapError::MissingDatabaseDirectory(_))
    ));
}

fn test_config(
    mode: RuntimeMode,
    database_path: impl AsRef<str>,
    files_dir: impl AsRef<str>,
) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            mode,
            ..ServerConfig::default()
        },
        storage: StorageConfig {
            database_path: database_path.as_ref().to_owned(),
            files_dir: files_dir.as_ref().to_owned(),
            auto_migrate: true,
        },
    }
}

async fn query_i64(database: &sea_orm::DatabaseConnection, sql: &str, column: &str) -> i64 {
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

async fn query_string_vec(
    database: &sea_orm::DatabaseConnection,
    sql: &str,
    column: &str,
) -> Vec<String> {
    database
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_owned(),
        ))
        .await
        .expect("query should execute")
        .into_iter()
        .map(|row| row.try_get("", column).expect("column should decode"))
        .collect()
}

async fn item_template_field_names(
    database: &sea_orm::DatabaseConnection,
    template_name: &str,
) -> Vec<String> {
    database
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT field.field_name
            FROM stock_item_attribute_definitions field
            INNER JOIN stock_item_attribute_templates template ON template.id = field.template_id
            WHERE template.name = ?
            ORDER BY field.sort_order, field.id
            "#,
            [template_name.into()],
        ))
        .await
        .expect("template fields should query")
        .into_iter()
        .map(|row| row.try_get("", "field_name").expect("field should decode"))
        .collect()
}
