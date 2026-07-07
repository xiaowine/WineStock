//! auth 模块业务服务。
//!
//! 本模块属于 `auth` 业务层，负责登录、刷新、登出等会话认证流程。
//! 它会调用 `security` 提供的密码、JWT 和令牌工具，但不负责路由级鉴权装配。

use sea_orm::DatabaseConnection;
use winestock_shared::{
    AuthClientKind, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthTokenResponse,
};

use crate::{
    persistence::{
        entity::refresh_token,
        repository::{
            sqlite_now, sqlite_time_after_seconds, CreateRefreshToken, RbacRepository,
            RefreshTokenRepository, UserRepository,
        },
    },
    security::{
        hash_refresh_token, random_urlsafe, verify_password, AuthApiError,
        CURRENT_REFRESH_TOKEN_VERSION,
    },
    state::CoreState,
    users::service::load_user_response,
};

/// 执行登录用例，并返回 access/refresh token 与当前用户快照。
pub(crate) async fn login(
    state: &CoreState,
    request: AuthLoginRequest,
) -> Result<AuthTokenResponse, AuthApiError> {
    let users = UserRepository::new(state.database());
    let Some(user) = users.find_by_username(&request.username).await? else {
        return Err(AuthApiError::InvalidCredentials);
    };
    if user.status != "active" || !verify_password(&request.password, &user.password_hash) {
        return Err(AuthApiError::InvalidCredentials);
    }

    let rbac = RbacRepository::new(state.database());
    let user_response = load_user_response(&rbac, &user).await?;
    let access_token = state.security().issue_access_token(
        user.id,
        user_response.roles.clone(),
        user_response.permissions.clone(),
    )?;
    let refresh_token = create_refresh_token(
        state,
        user.id,
        request.device_name,
        request.client_kind,
        request.version,
    )
    .await?;

    Ok(AuthTokenResponse {
        access_token,
        refresh_token,
        expires_in: state.security().settings().access_token_ttl_seconds,
        user: user_response,
    })
}

/// 执行 refresh token 轮换，并返回新的 token 与当前用户权限快照。
pub(crate) async fn refresh(
    state: &CoreState,
    request: AuthRefreshRequest,
) -> Result<AuthTokenResponse, AuthApiError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    let tokens = RefreshTokenRepository::new(state.database());
    let Some(existing) = tokens.find_by_hash(&token_hash).await? else {
        return Err(AuthApiError::InvalidRefreshToken);
    };
    reject_reused_or_expired_refresh_token(state.database(), &tokens, &existing).await?;

    let users = UserRepository::new(state.database());
    let Some(user) = users.find_by_id(existing.user_id).await? else {
        return Err(AuthApiError::InvalidRefreshToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidRefreshToken);
    }

    let rbac = RbacRepository::new(state.database());
    let user_response = load_user_response(&rbac, &user).await?;
    let plain_refresh_token = random_urlsafe(32).map_err(AuthApiError::Random)?;
    let new_hash = hash_refresh_token(&plain_refresh_token);
    let expires_at = sqlite_time_after_seconds(
        state.database(),
        state.security().settings().refresh_token_ttl_seconds,
    )
    .await?;
    let rotated = tokens
        .rotate(
            &token_hash,
            CreateRefreshToken {
                user_id: user.id,
                token_hash: new_hash,
                device_name: existing.device_name,
                client_kind: existing.client_kind,
                app_version: existing.app_version,
                refresh_token_version: CURRENT_REFRESH_TOKEN_VERSION.to_owned(),
                expires_at,
            },
        )
        .await?
        .ok_or(AuthApiError::InvalidRefreshToken)?;
    let access_token = state.security().issue_access_token(
        user.id,
        user_response.roles.clone(),
        user_response.permissions.clone(),
    )?;

    debug_assert_eq!(rotated.user_id, user.id);
    Ok(AuthTokenResponse {
        access_token,
        refresh_token: plain_refresh_token,
        expires_in: state.security().settings().access_token_ttl_seconds,
        user: user_response,
    })
}

/// 执行登出用例，成功时吊销当前 refresh token。
pub(crate) async fn logout(
    state: &CoreState,
    request: AuthLogoutRequest,
) -> Result<(), AuthApiError> {
    let token_hash = hash_refresh_token(&request.refresh_token);
    let tokens = RefreshTokenRepository::new(state.database());
    let revoked = tokens.revoke(&token_hash).await?;
    if !revoked {
        return Err(AuthApiError::InvalidRefreshToken);
    }

    Ok(())
}

/// 创建 refresh token 记录，并只把明文返回给调用方一次。
async fn create_refresh_token(
    state: &CoreState,
    user_id: i64,
    device_name: String,
    client_kind: AuthClientKind,
    app_version: String,
) -> Result<String, AuthApiError> {
    let plain_token = random_urlsafe(32).map_err(AuthApiError::Random)?;
    let token_hash = hash_refresh_token(&plain_token);
    let expires_at = sqlite_time_after_seconds(
        state.database(),
        state.security().settings().refresh_token_ttl_seconds,
    )
    .await?;
    RefreshTokenRepository::new(state.database())
        .create(CreateRefreshToken {
            user_id,
            token_hash,
            device_name,
            client_kind: client_kind.as_str().to_owned(),
            app_version,
            refresh_token_version: CURRENT_REFRESH_TOKEN_VERSION.to_owned(),
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
