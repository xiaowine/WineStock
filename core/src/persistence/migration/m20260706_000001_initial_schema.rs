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
        for statement in DROP_SCHEMA {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }

        Ok(())
    }
}

const INITIAL_SCHEMA: &[&str] = &[
    // 用户、角色和权限表先落地基础 RBAC 结构，后续 API 可以在此基础上补业务约束。
    r#"
    CREATE TABLE IF NOT EXISTS users (
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
    CREATE TABLE IF NOT EXISTS roles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        description TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS user_roles (
        user_id INTEGER NOT NULL,
        role_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (user_id, role_id),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
        FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS permissions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        description TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS role_permissions (
        role_id INTEGER NOT NULL,
        permission_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (role_id, permission_id),
        FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
        FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
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
    CREATE TABLE IF NOT EXISTS refresh_tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        device_name TEXT,
        client_kind TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        expires_at TEXT NOT NULL,
        last_used_at TEXT,
        revoked_at TEXT,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id
        ON refresh_tokens(user_id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_hash
        ON refresh_tokens(token_hash)
        WHERE revoked_at IS NULL
    "#,
    // 文件内容存放在 files/ 目录，SQLite 只记录可查询和可校验的元数据。
    r#"
    CREATE TABLE IF NOT EXISTS file_objects (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sha256 TEXT NOT NULL,
        mime_type TEXT,
        size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
        storage_path TEXT NOT NULL,
        original_name TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        owner_user_id INTEGER,
        FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE SET NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_file_objects_sha256
        ON file_objects(sha256)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_file_objects_owner_created
        ON file_objects(owner_user_id, created_at DESC)
    "#,
];

const DROP_SCHEMA: &[&str] = &[
    "DROP TABLE IF EXISTS file_objects",
    "DROP TABLE IF EXISTS refresh_tokens",
    "DROP INDEX IF EXISTS idx_auth_signing_keys_single_active",
    "DROP TABLE IF EXISTS auth_signing_keys",
    "DROP TABLE IF EXISTS auth_settings",
    "DROP TABLE IF EXISTS role_permissions",
    "DROP TABLE IF EXISTS permissions",
    "DROP TABLE IF EXISTS user_roles",
    "DROP TABLE IF EXISTS roles",
    "DROP TABLE IF EXISTS users",
];
