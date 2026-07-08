//! JWT 运行时支持。
//!
//! 本模块属于 `security` 前置层，负责 access token 的签发和校验。
//! 它不处理登录表单、注册表单或 refresh token 存储。

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthBootstrap, AuthSettings, AuthSigningKey};

use super::{
    current_user::CurrentUser,
    token::{random_urlsafe, unix_timestamp},
    AuthApiError,
};

/// 安全前置层请求处理中需要的 JWT 运行时快照。
#[derive(Debug, Clone)]
pub(crate) struct SecurityRuntime {
    settings: AuthSettings,
    active_signing_key: AuthSigningKey,
}

impl SecurityRuntime {
    /// 从鉴权 bootstrap 结果提取安全前置层请求处理所需状态。
    pub(crate) fn from_auth_bootstrap(auth: &AuthBootstrap) -> Self {
        Self {
            settings: auth.settings.clone(),
            active_signing_key: auth.active_signing_key.clone(),
        }
    }

    /// 返回当前使用的鉴权设置快照。
    pub(crate) fn settings(&self) -> &AuthSettings {
        &self.settings
    }

    /// 返回当前用于签发 access token 的 active 签名密钥。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn active_signing_key(&self) -> &AuthSigningKey {
        &self.active_signing_key
    }

    /// 根据用户当前权限快照签发短期 JWT access token。
    pub(crate) fn issue_access_token(
        &self,
        user_id: i64,
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
            permissions,
        };

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.active_signing_key.key_material.as_bytes()),
        )
        .map_err(AuthApiError::Jwt)
    }

    /// 校验 bearer token，并转换成处理函数可直接使用的当前用户上下文。
    pub(crate) fn verify_access_token(&self, token: &str) -> Result<CurrentUser, AuthApiError> {
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
            permissions: token_data.claims.permissions,
        })
    }
}

/// JWT access token 的服务端 claims。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccessClaims {
    /// 用户 ID，写入 JWT `sub`。
    pub(crate) sub: String,

    /// access token ID，便于后续审计或吊销能力扩展。
    pub(crate) jti: String,

    /// 签发时间，Unix 时间戳。
    pub(crate) iat: usize,

    /// 过期时间，Unix 时间戳。
    pub(crate) exp: usize,

    /// 签发时用户权限快照。
    pub(crate) permissions: Vec<String>,
}
