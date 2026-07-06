//! 鉴权运行时和当前用户提取。
//!
//! 本模块属于 core 鉴权层，负责 JWT access token 的签发、校验，以及 Axum
//! extractor 所需的当前用户上下文。它不处理登录表单或 refresh token 入库。

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

use crate::bootstrap::LocalServiceBootstrap;

use super::{
    bootstrap::{AuthSettings, AuthSigningKey},
    error::AuthApiError,
    security::{random_urlsafe, unix_timestamp},
};

/// 鉴权 HTTP 路由使用的共享状态。
///
/// 该状态属于 core 运行时，不暴露给平台 shell；签名密钥只用于服务端签发和校验 JWT。
#[derive(Debug, Clone)]
pub(crate) struct AuthRuntime {
    pub(super) database: DatabaseConnection,
    pub(super) settings: AuthSettings,
    pub(super) active_signing_key: AuthSigningKey,
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
    pub(super) fn issue_access_token(
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
    pub(super) fn verify_access_token(&self, token: &str) -> Result<CurrentUser, AuthApiError> {
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
pub(super) struct AccessClaims {
    /// 用户 ID，写入 JWT `sub`。
    pub(super) sub: String,

    /// access token ID，便于后续审计或吊销能力扩展。
    pub(super) jti: String,

    /// 签发时间，Unix 时间戳。
    pub(super) iat: usize,

    /// 过期时间，Unix 时间戳。
    pub(super) exp: usize,

    /// 签发时用户角色快照。
    pub(super) roles: Vec<String>,

    /// 签发时用户权限快照。
    pub(super) permissions: Vec<String>,
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
        if let Some(current_user) = parts.extensions.get::<CurrentUser>() {
            return Ok(current_user.clone());
        }

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
}

/// 从请求头解析 Bearer token，其他认证格式一律拒绝。
fn bearer_token(parts: &Parts) -> Option<&str> {
    bearer_token_from_headers(&parts.headers)
}

/// 从请求头集合解析 Bearer token，供 extractor 和条件鉴权接口共用。
pub(super) fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.trim().is_empty() {
        Some(token.trim())
    } else {
        None
    }
}
