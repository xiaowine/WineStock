//! core 的鉴权启动状态初始化。
//!
//! 本模块属于 `core axum library` 层，负责读取数据库托管的鉴权设置、
//! 准备 JWT access token 签名密钥，并提供登录、刷新、登出和当前用户提取逻辑。
//! 角色和权限基础定义属于 RBAC 模块；本模块只消费用户的角色/权限快照。
//! 它不拥有平台交互流程，也不把签名密钥或 refresh token 明文暴露给平台配置。

use std::{
    error::Error,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{DatabaseConnection, DbErr};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use winestock_shared::{
    AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};

use crate::{
    bootstrap::LocalServiceBootstrap,
    persistence::{
        entity::{auth_signing_key, refresh_token, user},
        repository::{
            sqlite_now, sqlite_time_after_seconds, AuthRepository, CreateRefreshToken, CreateUser,
            RbacRepository, RefreshTokenRepository, UserRepository,
        },
    },
    rbac::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME},
};

const REGISTER_USER_PERMISSION: &str = "user.register";

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

// 以下常量是数据库托管的鉴权设置键，不属于平台 JSON 启动配置。
const ACCESS_TOKEN_TTL_SECONDS: &str = "access_token_ttl_seconds";
const REFRESH_TOKEN_TTL_SECONDS: &str = "refresh_token_ttl_seconds";
const REFRESH_TOKEN_ROTATION: &str = "refresh_token_rotation";

// 当前仅生成对称签名密钥；若以后支持非对称算法，需要同步调整密钥材料存储语义。
const SIGNING_ALGORITHM: &str = "HS256";

// 缺省鉴权设置只用于补齐空库，不能覆盖数据库中已有的管理员配置。
const DEFAULT_AUTH_SETTINGS: [(&str, &str); 3] = [
    (ACCESS_TOKEN_TTL_SECONDS, "900"),
    (REFRESH_TOKEN_TTL_SECONDS, "604800"),
    (REFRESH_TOKEN_ROTATION, "true"),
];

/// 初始化本地服务的鉴权运行时状态。
///
/// 默认设置只在数据库缺失时插入，已有数据库值必须保留。
/// JWT 签名密钥也由数据库托管，缺少 active 密钥时才创建新密钥。
pub(crate) async fn bootstrap_auth(
    database: &DatabaseConnection,
) -> Result<AuthBootstrap, AuthBootstrapError> {
    let repository = AuthRepository::new(database);

    // 鉴权配置和签名密钥属于服务内部状态，不进入 JSON 启动配置。
    repository
        .insert_default_settings(&DEFAULT_AUTH_SETTINGS)
        .await?;
    let settings = load_auth_settings(&repository).await?;
    let active_signing_key = match repository.find_active_signing_key().await? {
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

/// 鉴权 HTTP 路由使用的共享状态。
///
/// 该状态属于 core 运行时，不暴露给平台 shell；签名密钥只用于服务端签发和校验 JWT。
#[derive(Debug, Clone)]
pub(crate) struct AuthRuntime {
    database: DatabaseConnection,
    settings: AuthSettings,
    active_signing_key: AuthSigningKey,
}

impl AuthRuntime {
    /// 从本地服务启动结果提取鉴权处理函数所需状态。
    pub(crate) fn from_local_service(local_service: &LocalServiceBootstrap) -> Self {
        Self {
            database: local_service.storage.database.clone(),
            settings: local_service.auth.settings.clone(),
            active_signing_key: local_service.auth.active_signing_key.clone(),
        }
    }

    /// 根据用户当前 RBAC 快照签发短期 JWT access token。
    fn issue_access_token(
        &self,
        user_id: i64,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<String, AuthApiError> {
        let now = unix_timestamp()?;
        let expires_at = now
            .checked_add(self.settings.access_token_ttl_seconds)
            .ok_or(AuthApiError::Internal)?;
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.active_signing_key.key_id.clone());
        let claims = AccessClaims {
            sub: user_id.to_string(),
            jti: random_urlsafe(16).map_err(AuthApiError::Random)?,
            iat: now as usize,
            exp: expires_at as usize,
            roles,
            permissions,
        };

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.active_signing_key.key_material.as_bytes()),
        )
        .map_err(AuthApiError::Jwt)
    }

    /// 校验 Bearer access token，并转换成处理函数可直接使用的当前用户上下文。
    fn verify_access_token(&self, token: &str) -> Result<CurrentUser, AuthApiError> {
        let header =
            jsonwebtoken::decode_header(token).map_err(|_| AuthApiError::InvalidAccessToken)?;
        if header.kid.as_deref() != Some(self.active_signing_key.key_id.as_str()) {
            return Err(AuthApiError::InvalidAccessToken);
        }

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let token_data = decode::<AccessClaims>(
            token,
            &DecodingKey::from_secret(self.active_signing_key.key_material.as_bytes()),
            &validation,
        )
        .map_err(|_| AuthApiError::InvalidAccessToken)?;
        let user_id = token_data
            .claims
            .sub
            .parse::<i64>()
            .map_err(|_| AuthApiError::InvalidAccessToken)?;

        Ok(CurrentUser {
            user_id,
            access_token_id: token_data.claims.jti,
            roles: token_data.claims.roles,
            permissions: token_data.claims.permissions,
        })
    }
}

/// JWT access token 的服务端 claims。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    /// 用户 ID，写入 JWT `sub`。
    sub: String,

    /// access token ID，便于后续审计或吊销能力扩展。
    jti: String,

    /// 签发时间，Unix 时间戳。
    iat: usize,

    /// 过期时间，Unix 时间戳。
    exp: usize,

    /// 签发时用户角色快照。
    roles: Vec<String>,

    /// 签发时用户权限快照。
    permissions: Vec<String>,
}

/// 已通过 Bearer access token 校验的当前用户上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUser {
    /// 数据库用户 ID。
    pub user_id: i64,

    /// JWT `jti`，当前用于审计上下文，不作为 refresh token 状态。
    pub access_token_id: String,

    /// access token 中携带的角色快照。
    pub roles: Vec<String>,

    /// access token 中携带的权限快照。
    pub permissions: Vec<String>,
}

impl FromRequestParts<AuthRuntime> for CurrentUser {
    type Rejection = AuthApiError;

    /// 从 `Authorization: Bearer` 请求头提取并校验 access token。
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AuthRuntime,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or(AuthApiError::InvalidAccessToken)?;
        state.verify_access_token(token)
    }
}

impl CurrentUser {
    /// 判断 access token claims 中是否包含指定权限。
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions
            .iter()
            .any(|candidate| candidate == permission)
    }

    /// 供受保护路由在 extractor 之后执行权限检查，缺少权限时统一返回 403。
    pub fn require_permission(&self, permission: &str) -> Result<(), AuthApiError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthApiError::PermissionDenied)
        }
    }
}

/// 鉴权 HTTP API 的错误响应。
#[derive(Debug)]
pub enum AuthApiError {
    /// 注册请求字段不满足服务端约束。
    InvalidRegisterRequest,

    /// 用户名已经存在。
    UsernameTaken,

    /// 用户名或密码错误，响应不暴露具体失败点。
    InvalidCredentials,

    /// refresh token 不存在、过期、已吊销或复用。
    InvalidRefreshToken,

    /// access token 缺失、格式错误、过期或签名无效。
    InvalidAccessToken,

    /// 当前用户缺少访问资源所需权限。
    PermissionDenied,

    /// 数据库读写失败。
    Database(DbErr),

    /// JWT 编码失败。
    Jwt(jsonwebtoken::errors::Error),

    /// 安全随机数生成失败。
    Random(getrandom::Error),

    /// 系统时间异常或内部状态不一致。
    Internal,
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRegisterRequest => (StatusCode::BAD_REQUEST, "invalid_register_request"),
            Self::UsernameTaken => (StatusCode::CONFLICT, "username_taken"),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            Self::InvalidRefreshToken => (StatusCode::UNAUTHORIZED, "invalid_refresh_token"),
            Self::InvalidAccessToken => (StatusCode::UNAUTHORIZED, "invalid_access_token"),
            Self::PermissionDenied => (StatusCode::FORBIDDEN, "permission_denied"),
            Self::Database(_) | Self::Jwt(_) | Self::Random(_) | Self::Internal => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_auth_error")
            }
        }
        .into_response()
    }
}

impl From<DbErr> for AuthApiError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

/// 注册新用户；首个用户免鉴权并自动成为 admin，之后必须拥有注册用户权限。
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = AuthRegisterRequest,
    security(
        (),
        ("bearerAuth" = [])
    ),
    responses(
        (status = 201, description = "User registered", body = AuthUserResponse),
        (status = 400, description = "Invalid register request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Register permission required", body = String),
        (status = 409, description = "Username already exists", body = String)
    )
)]
pub(crate) async fn register(
    State(state): State<AuthRuntime>,
    headers: HeaderMap,
    Json(request): Json<AuthRegisterRequest>,
) -> Result<(StatusCode, Json<AuthUserResponse>), AuthApiError> {
    let username = normalize_username(&request.username)?;
    if request.password.is_empty() {
        return Err(AuthApiError::InvalidRegisterRequest);
    }
    let auth_repository = AuthRepository::new(&state.database);
    let users = UserRepository::new(&state.database);
    let rbac = RbacRepository::new(&state.database);
    let has_users = auth_repository.has_any_user().await?;

    if has_users {
        require_current_permission_header(&state, &headers, REGISTER_USER_PERMISSION).await?;
    }
    if users.find_by_username(&username).await?.is_some() {
        return Err(AuthApiError::UsernameTaken);
    }

    let user = users
        .create_user(CreateUser {
            username,
            password_hash: create_password_hash(&request.password)?,
            display_name: None,
        })
        .await?;

    if !has_users {
        let admin_role_id = rbac
            .ensure_role(
                ADMIN_ROLE_CODE,
                ADMIN_ROLE_NAME,
                Some("系统管理员，拥有全部内置权限。"),
            )
            .await?;
        rbac.assign_role_to_user(user.id, admin_role_id).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(load_user_response(&rbac, &user).await?),
    ))
}

/// 用户名密码登录，成功后返回 JWT access token 和 opaque refresh token。
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = AuthLoginRequest,
    responses(
        (status = 200, description = "Login succeeded", body = AuthTokenResponse),
        (status = 401, description = "Invalid credentials", body = String)
    )
)]
pub(crate) async fn login(
    State(state): State<AuthRuntime>,
    Json(request): Json<AuthLoginRequest>,
) -> Result<Json<AuthTokenResponse>, AuthApiError> {
    let users = UserRepository::new(&state.database);
    let Some(user) = users.find_by_username(&request.username).await? else {
        return Err(AuthApiError::InvalidCredentials);
    };
    if user.status != "active" || !verify_password(&request.password, &user.password_hash) {
        return Err(AuthApiError::InvalidCredentials);
    }

    let rbac = RbacRepository::new(&state.database);
    let user_response = load_user_response(&rbac, &user).await?;
    let access_token = state.issue_access_token(
        user.id,
        user_response.roles.clone(),
        user_response.permissions.clone(),
    )?;
    let refresh_token =
        create_refresh_token(&state, user.id, request.device_name, request.client_kind).await?;

    Ok(Json(AuthTokenResponse {
        access_token,
        refresh_token,
        expires_in: state.settings.access_token_ttl_seconds,
        user: user_response,
    }))
}

/// 使用 refresh token 轮换并签发新的 access token。
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = AuthRefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = AuthTokenResponse),
        (status = 401, description = "Invalid refresh token", body = String)
    )
)]
pub(crate) async fn refresh(
    State(state): State<AuthRuntime>,
    Json(request): Json<AuthRefreshRequest>,
) -> Result<Json<AuthTokenResponse>, AuthApiError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    let tokens = RefreshTokenRepository::new(&state.database);
    let Some(existing) = tokens.find_by_hash(&token_hash).await? else {
        return Err(AuthApiError::InvalidRefreshToken);
    };
    reject_reused_or_expired_refresh_token(&state.database, &tokens, &existing).await?;

    let users = UserRepository::new(&state.database);
    let Some(user) = users.find_by_id(existing.user_id).await? else {
        return Err(AuthApiError::InvalidRefreshToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidRefreshToken);
    }

    let rbac = RbacRepository::new(&state.database);
    let user_response = load_user_response(&rbac, &user).await?;
    let plain_refresh_token = random_urlsafe(32).map_err(AuthApiError::Random)?;
    let new_hash = hash_refresh_token(&plain_refresh_token);
    let expires_at =
        sqlite_time_after_seconds(&state.database, state.settings.refresh_token_ttl_seconds)
            .await?;
    let rotated = tokens
        .rotate(
            &token_hash,
            CreateRefreshToken {
                user_id: user.id,
                token_hash: new_hash,
                device_name: existing.device_name,
                client_kind: existing.client_kind,
                expires_at,
            },
        )
        .await?
        .ok_or(AuthApiError::InvalidRefreshToken)?;
    let access_token = state.issue_access_token(
        user.id,
        user_response.roles.clone(),
        user_response.permissions.clone(),
    )?;

    debug_assert_eq!(rotated.user_id, user.id);
    Ok(Json(AuthTokenResponse {
        access_token,
        refresh_token: plain_refresh_token,
        expires_in: state.settings.access_token_ttl_seconds,
        user: user_response,
    }))
}

/// 吊销当前 refresh token；access token 自身仍按短 TTL 自然过期。
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    request_body = AuthLogoutRequest,
    responses(
        (status = 204, description = "Logged out"),
        (status = 401, description = "Invalid refresh token", body = String)
    )
)]
pub(crate) async fn logout(
    State(state): State<AuthRuntime>,
    Json(request): Json<AuthLogoutRequest>,
) -> Result<StatusCode, AuthApiError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    let tokens = RefreshTokenRepository::new(&state.database);
    let revoked = tokens.revoke(&token_hash).await?;
    if !revoked {
        return Err(AuthApiError::InvalidRefreshToken);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// 返回 Bearer access token 对应的当前用户。
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Current user", body = AuthUserResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Permission denied", body = String)
    )
)]
pub(crate) async fn me(
    State(state): State<AuthRuntime>,
    current_user: CurrentUser,
) -> Result<Json<AuthUserResponse>, AuthApiError> {
    let users = UserRepository::new(&state.database);
    let Some(user) = users.find_by_id(current_user.user_id).await? else {
        return Err(AuthApiError::InvalidAccessToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidAccessToken);
    }

    let rbac = RbacRepository::new(&state.database);
    Ok(Json(load_user_response(&rbac, &user).await?))
}

/// 根据明文密码和 PHC 格式 Argon2 哈希验证登录请求。
fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// 创建 refresh token 记录，并只把明文返回给调用方一次。
async fn create_refresh_token(
    state: &AuthRuntime,
    user_id: i64,
    device_name: Option<String>,
    client_kind: Option<String>,
) -> Result<String, AuthApiError> {
    let plain_token = random_urlsafe(32).map_err(AuthApiError::Random)?;
    let token_hash = hash_refresh_token(&plain_token);
    let expires_at =
        sqlite_time_after_seconds(&state.database, state.settings.refresh_token_ttl_seconds)
            .await?;
    RefreshTokenRepository::new(&state.database)
        .create(CreateRefreshToken {
            user_id,
            token_hash,
            device_name,
            client_kind,
            expires_at,
        })
        .await?;

    Ok(plain_token)
}

/// 拒绝已吊销、已过期或被复用的 refresh token。
async fn reject_reused_or_expired_refresh_token(
    database: &DatabaseConnection,
    tokens: &RefreshTokenRepository<'_>,
    token: &refresh_token::Model,
) -> Result<(), AuthApiError> {
    if token.revoked_at.is_some() {
        return Err(AuthApiError::InvalidRefreshToken);
    }

    let now = sqlite_now(database).await?;
    if token.expires_at <= now {
        tokens.revoke(&token.token_hash).await?;
        return Err(AuthApiError::InvalidRefreshToken);
    }

    Ok(())
}

/// 组装 API 返回和 JWT claims 共享的用户、角色、权限快照。
async fn load_user_response(
    rbac: &RbacRepository<'_>,
    user: &user::Model,
) -> Result<AuthUserResponse, AuthApiError> {
    Ok(AuthUserResponse {
        id: user.id.to_string(),
        username: user.username.clone(),
        roles: rbac.list_user_roles(user.id).await?,
        permissions: rbac.list_user_permissions(user.id).await?,
    })
}

/// 已有用户后注册新用户必须由当前仍拥有指定权限的 Bearer token 调用。
async fn require_current_permission_header(
    state: &AuthRuntime,
    headers: &HeaderMap,
    permission: &str,
) -> Result<CurrentUser, AuthApiError> {
    let token = bearer_token_from_headers(headers).ok_or(AuthApiError::InvalidAccessToken)?;
    let current_user = state.verify_access_token(token)?;
    let users = UserRepository::new(&state.database);
    let Some(user) = users.find_by_id(current_user.user_id).await? else {
        return Err(AuthApiError::InvalidAccessToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidAccessToken);
    }

    let rbac = RbacRepository::new(&state.database);
    let permissions = rbac.list_user_permissions(current_user.user_id).await?;
    if !permissions.iter().any(|candidate| candidate == permission) {
        return Err(AuthApiError::PermissionDenied);
    }

    Ok(current_user)
}

/// 生成 Argon2 PHC 密码哈希，注册流程之外不得保存明文密码。
fn create_password_hash(password: &str) -> Result<String, AuthApiError> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(AuthApiError::Random)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AuthApiError::Internal)?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthApiError::Internal)
}

/// 规范化用户名，避免空白用户名或仅靠首尾空白区分账号。
fn normalize_username(username: &str) -> Result<String, AuthApiError> {
    let username = username.trim();
    if username.is_empty() {
        Err(AuthApiError::InvalidRegisterRequest)
    } else {
        Ok(username.to_owned())
    }
}

/// 从请求头解析 Bearer token，其他认证格式一律拒绝。
fn bearer_token(parts: &Parts) -> Option<&str> {
    bearer_token_from_headers(&parts.headers)
}

/// 从请求头集合解析 Bearer token，供 extractor 和条件鉴权接口共用。
fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.trim().is_empty() {
        Some(token.trim())
    } else {
        None
    }
}

/// refresh token 使用高熵随机明文，数据库只保存其 SHA-256 哈希文本。
fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// 返回当前 Unix 时间戳，用于 JWT `iat` 和 `exp`。
fn unix_timestamp() -> Result<u64, AuthApiError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AuthApiError::Internal)
}

/// 从数据库加载完整鉴权策略，并把字符串设置解析成运行时类型。
async fn load_auth_settings(
    repository: &AuthRepository<'_>,
) -> Result<AuthSettings, AuthBootstrapError> {
    Ok(AuthSettings {
        access_token_ttl_seconds: parse_u64_setting(repository, ACCESS_TOKEN_TTL_SECONDS).await?,
        refresh_token_ttl_seconds: parse_u64_setting(repository, REFRESH_TOKEN_TTL_SECONDS).await?,
        refresh_token_rotation: parse_bool_setting(repository, REFRESH_TOKEN_ROTATION).await?,
    })
}

/// 读取秒数类鉴权设置；格式错误会阻止本地服务完成鉴权初始化。
async fn parse_u64_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<u64, AuthBootstrapError> {
    let value = require_setting(repository, key).await?;
    value
        .parse()
        .map_err(|_| AuthBootstrapError::InvalidSetting {
            key,
            value,
            expected: "unsigned integer seconds",
        })
}

/// 读取布尔类鉴权设置；数据库中只接受明确的 true/false 文本。
async fn parse_bool_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<bool, AuthBootstrapError> {
    let value = require_setting(repository, key).await?;

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

/// 读取必需的鉴权设置原始字符串；缺失表示 migration 或默认初始化没有成功。
async fn require_setting(
    repository: &AuthRepository<'_>,
    key: &'static str,
) -> Result<String, AuthBootstrapError> {
    repository
        .get_setting_value(key)
        .await?
        .ok_or(AuthBootstrapError::MissingSetting { key })
}

/// 创建首个 active 签名密钥；调用方只应在数据库不存在 active 密钥时使用。
async fn create_active_signing_key(
    repository: &AuthRepository<'_>,
) -> Result<auth_signing_key::Model, AuthBootstrapError> {
    // key_id 用于 JWT header 识别密钥，key_material 是真正签名材料，二者都由安全随机数生成。
    let key_id = format!("ak_{}", random_urlsafe(16)?);
    let key_material = random_urlsafe(32)?;

    repository
        .create_active_signing_key(key_id, SIGNING_ALGORITHM, key_material)
        .await
        .map_err(AuthBootstrapError::Database)
}

/// 把数据库模型转换成鉴权启动快照，并隔离数据库字段表示和运行时枚举。
fn signing_key_from_model(model: auth_signing_key::Model) -> AuthSigningKey {
    // 数据库 CHECK 约束已经限制状态值；未知值兜底为 retired，避免误当 active 使用。
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

/// 生成 URL-safe base64 随机文本，用于 JWT `kid` 和 HS256 对称签名密钥。
fn random_urlsafe(length: usize) -> Result<String, getrandom::Error> {
    // 使用 URL-safe base64，便于 key_id 和密钥材料进入 JSON/JWT 相关文本格式。
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use tempfile::{tempdir, TempDir};
    use tower::ServiceExt;
    use winestock_shared::{
        AppConfig, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, RuntimeMode,
        ServerConfig, StorageConfig,
    };

    use crate::{
        bootstrap_from_config,
        persistence::repository::{CreateUser, RbacRepository, UserRepository},
    };

    use super::*;

    struct TestApp {
        router: Router,
        state: AuthRuntime,
        _temp: TempDir,
    }

    #[tokio::test]
    async fn login_returns_tokens_and_access_token_reads_current_user() {
        let app = seeded_app().await;

        let login = login_request(&app, "admin", "password").await;

        assert_eq!(login.status, StatusCode::OK);
        assert!(!login.body.access_token.is_empty());
        assert!(!login.body.refresh_token.is_empty());
        assert_eq!(login.body.expires_in, 900);
        assert_eq!(login.body.user.username, "admin");
        assert_eq!(login.body.user.roles, vec!["admin"]);
        assert_eq!(
            login.body.user.permissions,
            vec!["stock.read", "stock.write", "user.manage", "user.register"]
        );

        let missing = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

        let me = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .header(
                        "authorization",
                        format!("Bearer {}", login.body.access_token),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(me.status(), StatusCode::OK);
        let current: AuthUserResponse = json_body(me).await;
        assert_eq!(current.username, "admin");
        assert_eq!(
            current.permissions,
            vec!["stock.read", "stock.write", "user.manage", "user.register"]
        );
    }

    #[tokio::test]
    async fn wrong_password_returns_uniform_unauthorized_error() {
        let app = seeded_app().await;
        let response = raw_login_request(&app, "admin", "wrong").await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(text_body(response).await, "invalid_credentials");
    }

    #[tokio::test]
    async fn first_registration_requires_no_token_and_becomes_admin() {
        let app = empty_app().await;

        let response = raw_register_request(&app, " first-admin ", "password", None).await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let user: AuthUserResponse = json_body(response).await;
        assert_eq!(user.username, "first-admin");
        assert_eq!(user.roles, vec!["admin"]);
        assert_eq!(
            user.permissions,
            vec!["stock.read", "stock.write", "user.manage", "user.register"]
        );

        let login = login_request(&app, "first-admin", "password").await;
        assert_eq!(login.status, StatusCode::OK);
        assert_eq!(login.body.user.roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn registration_requires_register_permission_after_first_user_exists() {
        let app = empty_app().await;
        let first = raw_register_request(&app, "admin", "password", None).await;
        assert_eq!(first.status(), StatusCode::CREATED);

        let missing_token = raw_register_request(&app, "staff", "password", None).await;
        assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

        seed_plain_user(&app.state.database, "plain", "password").await;
        let plain_login = login_request(&app, "plain", "password").await;
        assert_eq!(plain_login.status, StatusCode::OK);
        assert!(plain_login.body.user.roles.is_empty());

        let forbidden = raw_register_request(
            &app,
            "staff",
            "password",
            Some(&plain_login.body.access_token),
        )
        .await;
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

        let admin_login = login_request(&app, "admin", "password").await;
        let created = raw_register_request(
            &app,
            "staff",
            "password",
            Some(&admin_login.body.access_token),
        )
        .await;
        assert_eq!(created.status(), StatusCode::CREATED);
        let user: AuthUserResponse = json_body(created).await;
        assert_eq!(user.username, "staff");
        assert!(user.roles.is_empty());
    }

    #[tokio::test]
    async fn registration_allows_non_admin_role_with_register_permission() {
        let app = empty_app().await;
        let first = raw_register_request(&app, "admin", "password", None).await;
        assert_eq!(first.status(), StatusCode::CREATED);

        seed_plain_user(&app.state.database, "registrar", "password").await;
        let rbac = RbacRepository::new(&app.state.database);
        let registrar_role = rbac
            .ensure_role("registrar", "Registrar", Some("允许注册用户的业务角色。"))
            .await
            .expect("registrar role should exist");
        let register_permission = rbac
            .ensure_permission(REGISTER_USER_PERMISSION, Some("注册新用户。"))
            .await
            .expect("register permission should exist");
        rbac.assign_permission_to_role(registrar_role, register_permission)
            .await
            .expect("register permission should assign");
        let users = UserRepository::new(&app.state.database);
        let registrar = users
            .find_by_username("registrar")
            .await
            .expect("registrar lookup should succeed")
            .expect("registrar should exist");
        rbac.assign_role_to_user(registrar.id, registrar_role)
            .await
            .expect("registrar role should assign");

        let registrar_login = login_request(&app, "registrar", "password").await;
        assert_eq!(registrar_login.status, StatusCode::OK);
        assert_eq!(registrar_login.body.user.roles, vec!["registrar"]);
        assert_eq!(
            registrar_login.body.user.permissions,
            vec![REGISTER_USER_PERMISSION.to_owned()]
        );

        let created = raw_register_request(
            &app,
            "created-by-registrar",
            "password",
            Some(&registrar_login.body.access_token),
        )
        .await;

        assert_eq!(created.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn registration_checks_current_register_permission_in_database() {
        let app = empty_app().await;
        let first = raw_register_request(&app, "admin", "password", None).await;
        assert_eq!(first.status(), StatusCode::CREATED);
        let admin_login = login_request(&app, "admin", "password").await;

        app.state
            .database
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                DELETE FROM auth_user_role_assignments
                WHERE user_id = (
                    SELECT id FROM auth_users WHERE username = 'admin'
                )
                "#
                .to_owned(),
            ))
            .await
            .expect("admin role assignment should be removable");

        let stale_register_permission = raw_register_request(
            &app,
            "late-staff",
            "password",
            Some(&admin_login.body.access_token),
        )
        .await;

        assert_eq!(stale_register_permission.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn registration_rejects_duplicate_or_invalid_usernames() {
        let app = empty_app().await;
        let first = raw_register_request(&app, "admin", "password", None).await;
        assert_eq!(first.status(), StatusCode::CREATED);
        let admin_login = login_request(&app, "admin", "password").await;

        let duplicate = raw_register_request(
            &app,
            "admin",
            "password",
            Some(&admin_login.body.access_token),
        )
        .await;
        assert_eq!(duplicate.status(), StatusCode::CONFLICT);
        assert_eq!(text_body(duplicate).await, "username_taken");

        let empty_username = raw_register_request(
            &app,
            "   ",
            "password",
            Some(&admin_login.body.access_token),
        )
        .await;
        assert_eq!(empty_username.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn refresh_rotates_token_and_rejects_reused_old_token() {
        let app = seeded_app().await;
        let login = login_request(&app, "admin", "password").await;

        let first_refresh = refresh_request(&app, &login.body.refresh_token).await;
        assert_eq!(first_refresh.status, StatusCode::OK);
        assert_ne!(first_refresh.body.refresh_token, login.body.refresh_token);

        let reused_old = raw_refresh_request(&app, &login.body.refresh_token).await;
        assert_eq!(reused_old.status(), StatusCode::UNAUTHORIZED);

        let second_refresh = refresh_request(&app, &first_refresh.body.refresh_token).await;
        assert_eq!(second_refresh.status, StatusCode::OK);
        assert_ne!(
            second_refresh.body.refresh_token,
            first_refresh.body.refresh_token
        );
    }

    #[tokio::test]
    async fn refresh_rotation_keeps_other_device_tokens_active() {
        let app = seeded_app().await;
        let desktop_login = login_request(&app, "admin", "password").await;
        let android_login = login_request(&app, "admin", "password").await;
        assert_ne!(
            desktop_login.body.refresh_token,
            android_login.body.refresh_token
        );

        let desktop_refresh = refresh_request(&app, &desktop_login.body.refresh_token).await;
        assert_eq!(desktop_refresh.status, StatusCode::OK);

        let android_refresh = refresh_request(&app, &android_login.body.refresh_token).await;
        assert_eq!(android_refresh.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn logout_revokes_refresh_token() {
        let app = seeded_app().await;
        let login = login_request(&app, "admin", "password").await;

        let logout = app
            .router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/auth/logout",
                &AuthLogoutRequest {
                    refresh_token: login.body.refresh_token.clone(),
                },
            ))
            .await
            .expect("request should complete");
        assert_eq!(logout.status(), StatusCode::NO_CONTENT);

        let refresh = raw_refresh_request(&app, &login.body.refresh_token).await;
        assert_eq!(refresh.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_and_expired_access_tokens_are_rejected() {
        let app = seeded_app().await;

        let invalid = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .header("authorization", "Bearer not-a-jwt")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(app.state.active_signing_key.key_id.clone());
        let expired = encode(
            &header,
            &AccessClaims {
                sub: "1".to_owned(),
                jti: "expired".to_owned(),
                iat: 1,
                exp: 1,
                roles: vec![],
                permissions: vec![],
            },
            &EncodingKey::from_secret(app.state.active_signing_key.key_material.as_bytes()),
        )
        .expect("expired token should encode");

        let expired_response = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .header("authorization", format!("Bearer {expired}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);

        let mut wrong_key_header = Header::new(Algorithm::HS256);
        wrong_key_header.kid = Some(app.state.active_signing_key.key_id.clone());
        let wrong_signature = encode(
            &wrong_key_header,
            &AccessClaims {
                sub: "1".to_owned(),
                jti: "wrong-signature".to_owned(),
                iat: unix_timestamp().expect("time should be available") as usize,
                exp: (unix_timestamp().expect("time should be available") + 900) as usize,
                roles: vec![],
                permissions: vec![],
            },
            &EncodingKey::from_secret(b"wrong-signing-key"),
        )
        .expect("wrong-signature token should encode");

        let wrong_signature_response = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .header("authorization", format!("Bearer {wrong_signature}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");
        assert_eq!(wrong_signature_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn permission_helper_returns_forbidden_for_missing_permission() {
        async fn restricted(current_user: CurrentUser) -> Result<StatusCode, AuthApiError> {
            current_user.require_permission("admin.manage")?;
            Ok(StatusCode::NO_CONTENT)
        }

        let app = seeded_app().await;
        let login = login_request(&app, "admin", "password").await;
        let router = Router::new()
            .route("/restricted", get(restricted))
            .with_state(app.state.clone());

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/restricted")
                    .header(
                        "authorization",
                        format!("Bearer {}", login.body.access_token),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn openapi_includes_bearer_auth_and_auth_paths() {
        let app = seeded_app().await;
        let response = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(crate::OPENAPI_JSON_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value = json_body(response).await;
        assert!(value["components"]["securitySchemes"]["bearerAuth"].is_object());
        assert!(value["paths"]["/api/auth/register"].is_object());
        assert!(value["paths"]["/api/auth/login"].is_object());
        assert!(value["paths"]["/api/auth/refresh"].is_object());
        assert!(value["paths"]["/api/auth/logout"].is_object());
        assert!(value["paths"]["/api/auth/me"].is_object());
    }

    async fn seeded_app() -> TestApp {
        let app = empty_app().await;
        let users = UserRepository::new(&app.state.database);
        let rbac = RbacRepository::new(&app.state.database);
        let user = users
            .create_user(CreateUser {
                username: "admin".to_owned(),
                password_hash: password_hash("password"),
                display_name: Some("Admin".to_owned()),
            })
            .await
            .expect("user should be created");
        let admin_role_id = rbac
            .ensure_role(ADMIN_ROLE_CODE, ADMIN_ROLE_NAME, None)
            .await
            .expect("admin role should exist");
        rbac.assign_role_to_user(user.id, admin_role_id)
            .await
            .expect("admin role should assign");

        app
    }

    async fn empty_app() -> TestApp {
        let temp = tempdir().expect("temp dir should exist");
        let config = AppConfig {
            server: ServerConfig {
                mode: RuntimeMode::SelfHosted,
                ..ServerConfig::default()
            },
            storage: StorageConfig {
                database_path: temp
                    .path()
                    .join("winestock.sqlite")
                    .to_string_lossy()
                    .into_owned(),
                files_dir: temp.path().join("files").to_string_lossy().into_owned(),
                auto_migrate: true,
            },
        };
        let bootstrap = bootstrap_from_config(&config)
            .await
            .expect("bootstrap should succeed");
        let local = bootstrap.local_service.expect("local service should exist");
        let state = AuthRuntime::from_local_service(&local);
        let router = crate::build_router_with_local_service(&local);

        TestApp {
            router,
            state,
            _temp: temp,
        }
    }

    async fn seed_plain_user(database: &DatabaseConnection, username: &str, password: &str) {
        UserRepository::new(database)
            .create_user(CreateUser {
                username: username.to_owned(),
                password_hash: password_hash(password),
                display_name: None,
            })
            .await
            .expect("plain user should be created");
    }

    fn password_hash(password: &str) -> String {
        let salt = SaltString::from_b64("d2luZXN0b2NrX3Rlc3Rfc2FsdA").expect("salt should decode");
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("password should hash")
            .to_string()
    }

    async fn login_request(app: &TestApp, username: &str, password: &str) -> TokenResult {
        let response = raw_login_request(app, username, password).await;
        let status = response.status();
        let body = json_body(response).await;

        TokenResult { status, body }
    }

    async fn raw_login_request(
        app: &TestApp,
        username: &str,
        password: &str,
    ) -> axum::response::Response {
        app.router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/auth/login",
                &AuthLoginRequest {
                    username: username.to_owned(),
                    password: password.to_owned(),
                    device_name: Some("test-device".to_owned()),
                    client_kind: Some("test".to_owned()),
                },
            ))
            .await
            .expect("request should complete")
    }

    async fn raw_register_request(
        app: &TestApp,
        username: &str,
        password: &str,
        access_token: Option<&str>,
    ) -> axum::response::Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header("content-type", "application/json");
        if let Some(access_token) = access_token {
            builder = builder.header("authorization", format!("Bearer {access_token}"));
        }

        app.router
            .clone()
            .oneshot(
                builder
                    .body(Body::from(
                        serde_json::to_vec(&AuthRegisterRequest {
                            username: username.to_owned(),
                            password: password.to_owned(),
                        })
                        .expect("body should serialize"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("request should complete")
    }

    async fn refresh_request(app: &TestApp, refresh_token: &str) -> TokenResult {
        let response = raw_refresh_request(app, refresh_token).await;
        let status = response.status();
        let body = json_body(response).await;

        TokenResult { status, body }
    }

    async fn raw_refresh_request(app: &TestApp, refresh_token: &str) -> axum::response::Response {
        app.router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/auth/refresh",
                &AuthRefreshRequest {
                    refresh_token: refresh_token.to_owned(),
                },
            ))
            .await
            .expect("request should complete")
    }

    fn json_request<T: serde::Serialize>(method: &str, uri: &str, body: &T) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(body).expect("body should serialize"),
            ))
            .expect("request should build")
    }

    async fn json_body<T: for<'de> serde::Deserialize<'de>>(
        response: axum::response::Response,
    ) -> T {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        serde_json::from_slice(&bytes).expect("body should decode")
    }

    async fn text_body(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        String::from_utf8(bytes.to_vec()).expect("body should be utf8")
    }

    struct TokenResult {
        status: StatusCode,
        body: AuthTokenResponse,
    }
}
