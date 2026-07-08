//! 当前用户上下文提取。
//!
//! 本模块属于 `security` 前置层，负责 bearer token 解析和 Axum extractor。
//! 它不负责登录、注册或当前用户响应组装。

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};

use crate::state::CoreState;

use super::AuthApiError;

/// 已通过 bearer token 校验的当前用户上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUser {
    /// 数据库用户 ID。
    pub user_id: i64,

    /// JWT `jti`，当前用于审计上下文，不作为 refresh token 状态。
    pub access_token_id: String,

    /// access token 中携带的权限快照。
    pub permissions: Vec<String>,

    /// 数据库当前是否要求该用户先修改临时密码。
    pub password_change_required: bool,
}

impl FromRequestParts<CoreState> for CurrentUser {
    type Rejection = AuthApiError;

    /// 从 `Authorization: Bearer` 请求头提取并校验 access token。
    async fn from_request_parts(
        parts: &mut Parts,
        state: &CoreState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(current_user) = parts.extensions.get::<CurrentUser>() {
            return Ok(current_user.clone());
        }

        let token = bearer_token(parts).ok_or(AuthApiError::InvalidAccessToken)?;
        state.security().verify_access_token(token)
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

/// 从请求头解析 bearer token。
fn bearer_token(parts: &Parts) -> Option<&str> {
    bearer_token_from_headers(&parts.headers)
}

/// 从请求头集合解析 bearer token，供 extractor 和条件鉴权接口共用。
pub(super) fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.trim().is_empty() {
        Some(token.trim())
    } else {
        None
    }
}
