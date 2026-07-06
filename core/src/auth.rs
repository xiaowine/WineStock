use std::{error::Error, fmt};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension};

/// 数据库中的鉴权策略快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSettings {
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
    pub refresh_token_rotation: bool,
}

/// JWT 访问令牌签名密钥状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningKeyStatus {
    Active,
    Retired,
}

/// 当前可用于签发 JWT 访问令牌的密钥。
#[derive(Clone, PartialEq, Eq)]
pub struct AuthSigningKey {
    pub id: i64,
    pub key_id: String,
    pub algorithm: String,
    pub key_material: String,
    pub status: SigningKeyStatus,
    pub created_at: String,
    pub activated_at: Option<String>,
    pub retired_at: Option<String>,
}

impl fmt::Debug for AuthSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthSigningKey")
            .field("id", &self.id)
            .field("key_id", &self.key_id)
            .field("algorithm", &self.algorithm)
            .field("key_material", &"[redacted]")
            .field("status", &self.status)
            .field("created_at", &self.created_at)
            .field("activated_at", &self.activated_at)
            .field("retired_at", &self.retired_at)
            .finish()
    }
}

/// 本地服务鉴权初始化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthBootstrap {
    pub settings: AuthSettings,
    pub active_signing_key: AuthSigningKey,
    pub has_users: bool,
    pub admin_setup_required: bool,
}

/// 鉴权内部配置初始化错误。
#[derive(Debug)]
pub enum AuthBootstrapError {
    Database(rusqlite::Error),
    Random(getrandom::Error),
    InvalidSetting {
        key: &'static str,
        value: String,
        expected: &'static str,
    },
}

impl fmt::Display for AuthBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => write!(f, "failed to initialize auth settings"),
            Self::Random(_) => write!(f, "failed to generate auth signing key material"),
            Self::InvalidSetting {
                key,
                value,
                expected,
            } => write!(
                f,
                "invalid auth setting {key}={value:?}; expected {expected}"
            ),
        }
    }
}

impl Error for AuthBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
            Self::Random(source) => Some(source),
            Self::InvalidSetting { .. } => None,
        }
    }
}

impl From<rusqlite::Error> for AuthBootstrapError {
    fn from(source: rusqlite::Error) -> Self {
        Self::Database(source)
    }
}

impl From<getrandom::Error> for AuthBootstrapError {
    fn from(source: getrandom::Error) -> Self {
        Self::Random(source)
    }
}

const ACCESS_TOKEN_TTL_SECONDS: &str = "access_token_ttl_seconds";
const REFRESH_TOKEN_TTL_SECONDS: &str = "refresh_token_ttl_seconds";
const REFRESH_TOKEN_ROTATION: &str = "refresh_token_rotation";
const SIGNING_ALGORITHM: &str = "HS256";

const DEFAULT_AUTH_SETTINGS: [(&str, &str); 3] = [
    (ACCESS_TOKEN_TTL_SECONDS, "900"),
    (REFRESH_TOKEN_TTL_SECONDS, "604800"),
    (REFRESH_TOKEN_ROTATION, "true"),
];

pub(crate) fn migrate_auth_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    // 鉴权配置和签名密钥属于服务内部状态，不进入 JSON 启动配置。
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS auth_settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );

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
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_signing_keys_single_active
            ON auth_signing_keys(status)
            WHERE status = 'active';
        "#,
    )
}

pub(crate) fn bootstrap_auth(conn: &Connection) -> Result<AuthBootstrap, AuthBootstrapError> {
    insert_default_settings(conn)?;
    let settings = read_auth_settings(conn)?;
    let active_signing_key = match active_signing_key(conn)? {
        Some(key) => key,
        None => create_active_signing_key(conn)?,
    };
    let has_users = has_any_user(conn)?;

    Ok(AuthBootstrap {
        settings,
        active_signing_key,
        has_users,
        admin_setup_required: !has_users,
    })
}

fn insert_default_settings(conn: &Connection) -> Result<(), rusqlite::Error> {
    for (key, value) in DEFAULT_AUTH_SETTINGS {
        conn.execute(
            r#"
            INSERT INTO auth_settings (key, value, updated_at)
            VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ON CONFLICT(key) DO NOTHING
            "#,
            params![key, value],
        )?;
    }

    Ok(())
}

fn read_auth_settings(conn: &Connection) -> Result<AuthSettings, AuthBootstrapError> {
    Ok(AuthSettings {
        access_token_ttl_seconds: read_u64_setting(conn, ACCESS_TOKEN_TTL_SECONDS)?,
        refresh_token_ttl_seconds: read_u64_setting(conn, REFRESH_TOKEN_TTL_SECONDS)?,
        refresh_token_rotation: read_bool_setting(conn, REFRESH_TOKEN_ROTATION)?,
    })
}

fn read_u64_setting(conn: &Connection, key: &'static str) -> Result<u64, AuthBootstrapError> {
    let value = read_setting(conn, key)?;
    value
        .parse()
        .map_err(|_| AuthBootstrapError::InvalidSetting {
            key,
            value,
            expected: "unsigned integer seconds",
        })
}

fn read_bool_setting(conn: &Connection, key: &'static str) -> Result<bool, AuthBootstrapError> {
    let value = read_setting(conn, key)?;

    match value.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(AuthBootstrapError::InvalidSetting {
            key,
            value,
            expected: "true or false",
        }),
    }
}

fn read_setting(conn: &Connection, key: &'static str) -> Result<String, AuthBootstrapError> {
    conn.query_row(
        "SELECT value FROM auth_settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .map_err(AuthBootstrapError::Database)
}

fn active_signing_key(conn: &Connection) -> Result<Option<AuthSigningKey>, rusqlite::Error> {
    conn.query_row(
        r#"
        SELECT id, key_id, algorithm, key_material, status, created_at, activated_at, retired_at
        FROM auth_signing_keys
        WHERE status = 'active'
        ORDER BY activated_at DESC, id DESC
        LIMIT 1
        "#,
        [],
        signing_key_from_row,
    )
    .optional()
}

fn create_active_signing_key(conn: &Connection) -> Result<AuthSigningKey, AuthBootstrapError> {
    let key_id = format!("ak_{}", random_urlsafe(16)?);
    let key_material = random_urlsafe(32)?;

    conn.execute(
        r#"
        INSERT INTO auth_signing_keys
            (key_id, algorithm, key_material, status, created_at, activated_at)
        VALUES
            (?1, ?2, ?3, 'active',
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        "#,
        params![key_id, SIGNING_ALGORITHM, key_material],
    )?;

    let row_id = conn.last_insert_rowid();
    conn.query_row(
        r#"
        SELECT id, key_id, algorithm, key_material, status, created_at, activated_at, retired_at
        FROM auth_signing_keys
        WHERE id = ?1
        "#,
        params![row_id],
        signing_key_from_row,
    )
    .map_err(AuthBootstrapError::Database)
}

fn signing_key_from_row(row: &rusqlite::Row<'_>) -> Result<AuthSigningKey, rusqlite::Error> {
    let status: String = row.get(4)?;
    let status = match status.as_str() {
        "active" => SigningKeyStatus::Active,
        "retired" => SigningKeyStatus::Retired,
        _ => SigningKeyStatus::Retired,
    };

    Ok(AuthSigningKey {
        id: row.get(0)?,
        key_id: row.get(1)?,
        algorithm: row.get(2)?,
        key_material: row.get(3)?,
        status,
        created_at: row.get(5)?,
        activated_at: row.get(6)?,
        retired_at: row.get(7)?,
    })
}

fn has_any_user(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let users_table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'users'",
        [],
        |row| row.get(0),
    )?;

    if users_table_count == 0 {
        return Ok(false);
    }

    let has_user: i64 =
        conn.query_row("SELECT EXISTS(SELECT 1 FROM users LIMIT 1)", [], |row| {
            row.get(0)
        })?;

    Ok(has_user != 0)
}

fn random_urlsafe(length: usize) -> Result<String, getrandom::Error> {
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
