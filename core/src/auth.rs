use std::{error::Error, fmt};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sea_orm::{DatabaseConnection, DbErr};

use crate::persistence::{entity::auth_signing_key, repository::AuthRepository};

/// 数据库中的鉴权策略快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSettings {
    /// 访问令牌有效期，单位秒。
    pub access_token_ttl_seconds: u64,

    /// 刷新令牌有效期，单位秒。
    pub refresh_token_ttl_seconds: u64,

    /// 是否启用刷新令牌轮换。
    pub refresh_token_rotation: bool,
}

/// JWT 访问令牌签名密钥状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningKeyStatus {
    /// 当前用于签发访问令牌的密钥。
    Active,

    /// 已退役、不可再用于新签发的密钥。
    Retired,
}

/// 当前可用于签发 JWT 访问令牌的密钥。
#[derive(Clone, PartialEq, Eq)]
pub struct AuthSigningKey {
    /// 数据库自增主键。
    pub id: i64,

    /// JWT 头部中使用的密钥标识。
    pub key_id: String,

    /// 签名算法标识，当前默认 HS256。
    pub algorithm: String,

    /// 签名密钥材料，不能写入日志或普通响应。
    pub key_material: String,

    /// 密钥生命周期状态。
    pub status: SigningKeyStatus,

    /// 密钥创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 密钥启用时间。
    pub activated_at: Option<String>,

    /// 密钥退役时间。
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
    /// 本次启动读取到的鉴权策略。
    pub settings: AuthSettings,

    /// 当前可用于签发访问令牌的 active 密钥。
    pub active_signing_key: AuthSigningKey,

    /// 数据库中是否已经存在用户。
    pub has_users: bool,

    /// 是否需要执行首次管理员初始化流程。
    pub admin_setup_required: bool,
}

/// 鉴权内部配置初始化错误。
#[derive(Debug)]
pub enum AuthBootstrapError {
    /// SeaORM 或 SQLite 查询失败。
    Database(DbErr),

    /// 生成签名密钥随机材料失败。
    Random(getrandom::Error),

    /// 数据库缺少必需的鉴权设置。
    MissingSetting {
        /// 缺失的设置键。
        key: &'static str,
    },

    /// 数据库中的鉴权设置值无法解析为期望类型。
    InvalidSetting {
        /// 设置键。
        key: &'static str,

        /// 数据库中的原始设置值。
        value: String,

        /// 期望的数据格式说明。
        expected: &'static str,
    },
}

impl fmt::Display for AuthBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => write!(f, "failed to initialize auth settings"),
            Self::Random(_) => write!(f, "failed to generate auth signing key material"),
            Self::MissingSetting { key } => {
                write!(f, "missing required auth setting {key}")
            }
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
            Self::MissingSetting { .. } | Self::InvalidSetting { .. } => None,
        }
    }
}

impl From<DbErr> for AuthBootstrapError {
    fn from(source: DbErr) -> Self {
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

pub(crate) async fn bootstrap_auth(
    database: &DatabaseConnection,
) -> Result<AuthBootstrap, AuthBootstrapError> {
    let repository = AuthRepository::new(database);

    // 鉴权配置和签名密钥属于服务内部状态，不进入 JSON 启动配置。
    repository
        .insert_default_settings(&DEFAULT_AUTH_SETTINGS)
        .await?;
    let settings = read_auth_settings(&repository).await?;
    let active_signing_key = match repository.active_signing_key().await? {
        Some(key) => key,
        None => create_active_signing_key(&repository).await?,
    };
    let has_users = repository.has_any_user().await?;

    Ok(AuthBootstrap {
        settings,
        active_signing_key: signing_key_from_model(active_signing_key),
        has_users,
        admin_setup_required: !has_users,
    })
}

async fn read_auth_settings(
    repository: &AuthRepository<'_>,
) -> Result<AuthSettings, AuthBootstrapError> {
    Ok(AuthSettings {
        access_token_ttl_seconds: read_u64_setting(repository, ACCESS_TOKEN_TTL_SECONDS).await?,
        refresh_token_ttl_seconds: read_u64_setting(repository, REFRESH_TOKEN_TTL_SECONDS).await?,
        refresh_token_rotation: read_bool_setting(repository, REFRESH_TOKEN_ROTATION).await?,
    })
}

async fn read_u64_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<u64, AuthBootstrapError> {
    let value = read_setting(repository, key).await?;
    value
        .parse()
        .map_err(|_| AuthBootstrapError::InvalidSetting {
            key,
            value,
            expected: "unsigned integer seconds",
        })
}

async fn read_bool_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<bool, AuthBootstrapError> {
    let value = read_setting(repository, key).await?;

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

async fn read_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<String, AuthBootstrapError> {
    repository
        .setting_value(key)
        .await?
        .ok_or(AuthBootstrapError::MissingSetting { key })
}

async fn create_active_signing_key(
    repository: &AuthRepository<'_>,
) -> Result<auth_signing_key::Model, AuthBootstrapError> {
    let key_id = format!("ak_{}", random_urlsafe(16)?);
    let key_material = random_urlsafe(32)?;

    repository
        .create_active_signing_key(key_id, SIGNING_ALGORITHM, key_material)
        .await
        .map_err(AuthBootstrapError::Database)
}

fn signing_key_from_model(model: auth_signing_key::Model) -> AuthSigningKey {
    let status = match model.status.as_str() {
        "active" => SigningKeyStatus::Active,
        "retired" => SigningKeyStatus::Retired,
        _ => SigningKeyStatus::Retired,
    };

    AuthSigningKey {
        id: model.id,
        key_id: model.key_id,
        algorithm: model.algorithm,
        key_material: model.key_material,
        status,
        created_at: model.created_at,
        activated_at: model.activated_at,
        retired_at: model.retired_at,
    }
}

fn random_urlsafe(length: usize) -> Result<String, getrandom::Error> {
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
