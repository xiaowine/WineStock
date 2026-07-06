//! WineStock v1 SQLite schema migration。
//!
//! 本 migration 创建当前 server/API 阶段需要的用户、鉴权、刷新令牌和文件元数据表。
//! 这里保留显式 SQL，是为了直接表达 SQLite CHECK、局部唯一索引和默认时间格式。

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

/// 创建 WineStock v1 本地 SQLite schema。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 首版 schema 使用显式 SQL，便于保留 SQLite CHECK、局部唯一索引和时间默认值。
        for statement in INITIAL_SCHEMA {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚按依赖反序删除索引和表，避免外键引用影响 schema 清理。
        for statement in DROP_SCHEMA {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }

        Ok(())
    }
}

// 首版 schema 的建表和索引语句，按外键依赖顺序执行。
const INITIAL_SCHEMA: &[&str] = &[
    // 鉴权账号、角色和权限表统一使用 auth_ 前缀，避免和 SQLite/SeaORM 系统表混淆。
    r#"
    CREATE TABLE IF NOT EXISTS auth_users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        display_name TEXT,
        status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_roles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        description TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_user_role_assignments (
        user_id INTEGER NOT NULL,
        role_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (user_id, role_id),
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE,
        FOREIGN KEY (role_id) REFERENCES auth_roles(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_permissions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        description TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_role_permission_assignments (
        role_id INTEGER NOT NULL,
        permission_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (role_id, permission_id),
        FOREIGN KEY (role_id) REFERENCES auth_roles(id) ON DELETE CASCADE,
        FOREIGN KEY (permission_id) REFERENCES auth_permissions(id) ON DELETE CASCADE
    )
    "#,
    // 鉴权策略和 JWT access token 签名密钥由数据库管理，不进入 JSON 启动配置。
    r#"
    CREATE TABLE IF NOT EXISTS auth_settings (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS auth_signing_keys (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        key_id TEXT NOT NULL UNIQUE,
        algorithm TEXT NOT NULL,
        key_material TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('active', 'retired')),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        activated_at TEXT,
        retired_at TEXT,
        CHECK (status != 'active' OR activated_at IS NOT NULL),
        CHECK (status != 'retired' OR retired_at IS NOT NULL)
    )
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_signing_keys_single_active
        ON auth_signing_keys(status)
        WHERE status = 'active'
    "#,
    // 刷新令牌保留设备来源和吊销时间，轮换逻辑必须在仓储层事务中完成。
    r#"
    CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        device_name TEXT,
        client_kind TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        expires_at TEXT NOT NULL,
        last_used_at TEXT,
        revoked_at TEXT,
        FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_user_id
        ON auth_refresh_tokens(user_id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_active_hash
        ON auth_refresh_tokens(token_hash)
        WHERE revoked_at IS NULL
    "#,
    // 文件内容存放在 files/ 目录，SQLite 只记录可查询和可校验的元数据。
    r#"
    CREATE TABLE IF NOT EXISTS storage_file_objects (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sha256 TEXT NOT NULL,
        mime_type TEXT,
        size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
        storage_path TEXT NOT NULL,
        original_name TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        owner_user_id INTEGER,
        FOREIGN KEY (owner_user_id) REFERENCES auth_users(id) ON DELETE SET NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_storage_file_objects_sha256
        ON storage_file_objects(sha256)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_storage_file_objects_owner_created
        ON storage_file_objects(owner_user_id, created_at DESC)
    "#,
];

// 首版 schema 的回滚语句，顺序必须与建表依赖相反。
const DROP_SCHEMA: &[&str] = &[
    "DROP INDEX IF EXISTS idx_storage_file_objects_owner_created",
    "DROP INDEX IF EXISTS idx_storage_file_objects_sha256",
    "DROP TABLE IF EXISTS storage_file_objects",
    "DROP INDEX IF EXISTS idx_auth_refresh_tokens_active_hash",
    "DROP INDEX IF EXISTS idx_auth_refresh_tokens_user_id",
    "DROP TABLE IF EXISTS auth_refresh_tokens",
    "DROP INDEX IF EXISTS idx_auth_signing_keys_single_active",
    "DROP TABLE IF EXISTS auth_signing_keys",
    "DROP TABLE IF EXISTS auth_settings",
    "DROP TABLE IF EXISTS auth_role_permission_assignments",
    "DROP TABLE IF EXISTS auth_permissions",
    "DROP TABLE IF EXISTS auth_user_role_assignments",
    "DROP TABLE IF EXISTS auth_roles",
    "DROP TABLE IF EXISTS auth_users",
];
