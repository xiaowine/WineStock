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
            "SELECT code FROM auth_roles ORDER BY code",
            "code",
        )
        .await,
        vec!["admin", "staff", "viewer"]
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT code FROM auth_permissions ORDER BY code",
            "code",
        )
        .await,
        vec!["stock.read", "stock.write", "user.manage", "user.register"]
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            r#"
            SELECT auth_permissions.code AS code
            FROM auth_permissions
            INNER JOIN auth_role_permission_assignments
                ON auth_role_permission_assignments.permission_id = auth_permissions.id
            INNER JOIN auth_roles
                ON auth_roles.id = auth_role_permission_assignments.role_id
            WHERE auth_roles.code = 'admin'
            ORDER BY auth_permissions.code
            "#,
            "code",
        )
        .await,
        vec!["stock.read", "stock.write", "user.manage", "user.register"]
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
        .execute(Statement::from_string(
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
async fn builtin_rbac_bootstrap_is_idempotent_and_preserves_existing_text() {
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
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE auth_roles SET name = '自定义管理员' WHERE code = 'admin'".to_owned(),
        ))
        .await
        .expect("role should update");

    let second = bootstrap_from_config(&config)
        .await
        .expect("second bootstrap should preserve rbac")
        .local_service
        .expect("local service should be initialized");

    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM auth_roles",
            "count",
        )
        .await,
        3
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM auth_permissions",
            "count",
        )
        .await,
        4
    );
    assert_eq!(
        query_i64(
            &second.storage.database,
            "SELECT COUNT(*) AS count FROM auth_role_permission_assignments",
            "count",
        )
        .await,
        7
    );
    assert_eq!(
        query_string_vec(
            &second.storage.database,
            "SELECT name FROM auth_roles WHERE code = 'admin'",
            "name",
        )
        .await,
        vec!["自定义管理员"]
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

async fn query_string_vec(
    database: &sea_orm::DatabaseConnection,
    sql: &str,
    column: &str,
) -> Vec<String> {
    database
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_owned(),
        ))
        .await
        .expect("query should execute")
        .into_iter()
        .map(|row| row.try_get("", column).expect("column should decode"))
        .collect()
}
