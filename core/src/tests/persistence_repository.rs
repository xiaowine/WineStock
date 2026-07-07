//! 持久化 repository 测试。

use std::fs;

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use tempfile::{tempdir, TempDir};
use winestock_shared::StorageConfig;

use crate::persistence::{migrate_storage_schema, open_sqlite_storage, StorageRuntime};

use super::{
    file_object::{CreateFileObject, FileObjectRepository},
    rbac_repo::{RbacRepository, RolePermissionSyncMode},
    refresh_token_repo::{CreateRefreshToken, RefreshTokenRepository},
    user_repo::{CreateUser, UserRepository},
};

struct TestStorage {
    storage: StorageRuntime,
    _temp: TempDir,
}

#[tokio::test]
async fn refresh_token_repository_handles_create_revoke_and_rotate_in_transactions() {
    let test_storage = migrated_storage().await;
    let storage = &test_storage.storage;
    let users = UserRepository::new(&storage.database);
    let user = users
        .create_user(CreateUser {
            username: "admin".to_owned(),
            password_hash: "password-hash".to_owned(),
            display_name: Some("Admin".to_owned()),
        })
        .await
        .expect("user should be created");
    let tokens = RefreshTokenRepository::new(&storage.database);

    tokens
        .create(CreateRefreshToken {
            user_id: user.id,
            token_hash: "hash-1".to_owned(),
            device_name: "desktop".to_owned(),
            client_kind: "desktop".to_owned(),
            app_version: "0.1.0-test".to_owned(),
            refresh_token_version: "v1".to_owned(),
            expires_at: "2099-01-01T00:00:00.000Z".to_owned(),
        })
        .await
        .expect("refresh token should be created");

    assert!(tokens
        .find_active_by_hash("hash-1")
        .await
        .expect("token lookup should succeed")
        .is_some());
    assert!(tokens
        .revoke("hash-1")
        .await
        .expect("token revoke should succeed"));
    assert!(tokens
        .find_active_by_hash("hash-1")
        .await
        .expect("token lookup should succeed")
        .is_none());

    tokens
        .create(CreateRefreshToken {
            user_id: user.id,
            token_hash: "hash-2".to_owned(),
            device_name: "desktop".to_owned(),
            client_kind: "desktop".to_owned(),
            app_version: "0.1.0-test".to_owned(),
            refresh_token_version: "v1".to_owned(),
            expires_at: "2099-01-01T00:00:00.000Z".to_owned(),
        })
        .await
        .expect("refresh token should be created");

    let rotated = tokens
        .rotate(
            "hash-2",
            CreateRefreshToken {
                user_id: user.id,
                token_hash: "hash-3".to_owned(),
                device_name: "desktop".to_owned(),
                client_kind: "desktop".to_owned(),
                app_version: "0.1.0-test".to_owned(),
                refresh_token_version: "v1".to_owned(),
                expires_at: "2099-01-01T00:00:00.000Z".to_owned(),
            },
        )
        .await
        .expect("token rotation should succeed")
        .expect("active token should rotate");

    assert_eq!(rotated.token_hash, "hash-3");
    assert!(tokens
        .find_active_by_hash("hash-2")
        .await
        .expect("token lookup should succeed")
        .is_none());
    assert!(tokens
        .find_active_by_hash("hash-3")
        .await
        .expect("token lookup should succeed")
        .is_some());
}

#[tokio::test]
async fn file_object_repository_stores_metadata_while_content_stays_in_files_dir() {
    let test_storage = migrated_storage().await;
    let storage = &test_storage.storage;
    fs::create_dir_all(&storage.files_dir).expect("files dir should be created");
    let content_path = storage.files_dir.join("sha256-example.bin");
    fs::write(&content_path, [0_u8, 1, 2, 3]).expect("file content should be written");

    let files = FileObjectRepository::new(&storage.database);
    let metadata = files
        .create_metadata(CreateFileObject {
            sha256: "sha256-example".to_owned(),
            mime_type: Some("application/octet-stream".to_owned()),
            size_bytes: 4,
            storage_path: "sha256-example.bin".to_owned(),
            original_name: Some("example.bin".to_owned()),
            owner_user_id: None,
        })
        .await
        .expect("file metadata should be created");

    assert_eq!(metadata.storage_path, "sha256-example.bin");
    assert_eq!(
        files
            .find_by_sha256("sha256-example")
            .await
            .expect("sha lookup should succeed")
            .len(),
        1
    );
    assert_eq!(
        fs::read(&content_path).expect("content should remain in files dir"),
        [0_u8, 1, 2, 3]
    );
}

#[tokio::test]
async fn rbac_role_permission_sync_can_preserve_or_replace_existing_permissions() {
    let test_storage = migrated_storage().await;
    let storage = &test_storage.storage;
    let rbac = RbacRepository::new(&storage.database);
    let role_id = rbac
        .ensure_role("ops", "Ops", Some("测试角色"))
        .await
        .expect("role should be created");
    let read_permission_id = rbac
        .ensure_permission("stock.read", Some("查看库存"))
        .await
        .expect("read permission should be created");
    let write_permission_id = rbac
        .ensure_permission("stock.write", Some("修改库存"))
        .await
        .expect("write permission should be created");
    let manage_permission_id = rbac
        .ensure_permission("user.manage", Some("管理用户"))
        .await
        .expect("manage permission should be created");

    rbac.assign_permission_to_role(role_id, read_permission_id)
        .await
        .expect("read permission should be assigned");
    rbac.sync_role_permissions(
        role_id,
        &[write_permission_id],
        RolePermissionSyncMode::PreserveExisting,
    )
    .await
    .expect("preserve sync should succeed");

    assert_eq!(
        role_permission_codes(storage, role_id).await,
        vec!["stock.read".to_owned(), "stock.write".to_owned()]
    );

    rbac.sync_role_permissions(
        role_id,
        &[manage_permission_id],
        RolePermissionSyncMode::ReplaceExisting,
    )
    .await
    .expect("replace sync should succeed");

    assert_eq!(
        role_permission_codes(storage, role_id).await,
        vec!["user.manage".to_owned()]
    );
}

#[tokio::test]
async fn wal_allows_reads_while_a_write_transaction_is_open() {
    let test_storage = migrated_storage().await;
    let storage = &test_storage.storage;
    let users = UserRepository::new(&storage.database);
    users
        .create_user(CreateUser {
            username: "reader".to_owned(),
            password_hash: "password-hash".to_owned(),
            display_name: None,
        })
        .await
        .expect("seed user should be created");

    let transaction = storage
        .database
        .begin()
        .await
        .expect("write transaction should begin");
    transaction
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO auth_users (username, password_hash, status, created_at, updated_at)
            VALUES ('writer', 'password-hash', 'active',
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            "#
            .to_owned(),
        ))
        .await
        .expect("uncommitted write should succeed");

    let visible_count =
        query_i64(storage, "SELECT COUNT(*) AS count FROM auth_users", "count").await;

    assert_eq!(visible_count, 1);
    transaction
        .rollback()
        .await
        .expect("write transaction should roll back");
}

async fn migrated_storage() -> TestStorage {
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
        .expect("migration should succeed");

    TestStorage {
        storage,
        _temp: temp,
    }
}

async fn query_i64(storage: &StorageRuntime, sql: &str, column: &str) -> i64 {
    storage
        .database
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

async fn role_permission_codes(storage: &StorageRuntime, role_id: i64) -> Vec<String> {
    storage
        .database
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT auth_permissions.code AS code
            FROM auth_permissions
            INNER JOIN auth_role_permission_assignments
                ON auth_role_permission_assignments.permission_id = auth_permissions.id
            WHERE auth_role_permission_assignments.role_id = ?
            ORDER BY auth_permissions.code
            "#,
            [role_id.into()],
        ))
        .await
        .expect("role permission query should execute")
        .into_iter()
        .map(|row| row.try_get("", "code").expect("code should decode"))
        .collect()
}
