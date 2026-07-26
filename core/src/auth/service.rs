//! auth 模块业务服务。
//!
//! 本模块属于 `auth` 业务层，负责登录、刷新、登出等会话认证流程。
//! 它会调用 `security` 提供的密码、JWT 和令牌工具，但不负责路由级鉴权装配。

use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};

use crate::{
    auth::{
        AuthClientKind, AuthLocalSessionRequest, AuthLocalSessionStatus, AuthLoginRequest,
        AuthLogoutRequest, AuthRefreshRequest, AuthTokenResponse,
    },
    persistence::{
        entity::{refresh_token, user},
        repository::{
            sqlite_now, sqlite_time_after_seconds, CreateRefreshToken, RbacRepository,
            RefreshTokenRepository, UserRepository,
        },
    },
    security::{
        hash_refresh_token, random_urlsafe, verify_password, AuthApiError, CurrentUser,
        CURRENT_REFRESH_TOKEN_VERSION,
    },
    state::CoreState,
    users::service::{
        load_user_response, password_placeholder_active, resolve_local_auto_login_user,
    },
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

    issue_session_response(
        state,
        &user,
        request.device_name,
        request.client_kind,
        request.version,
    )
    .await
}

/// self-hosted 本机静默会话换取：校验壳内下发的换取凭据后为标记用户签发正常 token 对。
///
/// 空库首次换取会自动开通默认管理员；标记用户被停用/收权时自愈。非 self-hosted 模式
/// 或存量库未标记换取目标时返回 `LocalSessionUnavailable`。
pub(crate) async fn local_session(
    state: &CoreState,
    request: AuthLocalSessionRequest,
) -> Result<AuthTokenResponse, AuthApiError> {
    let Some(secret) = state.local_session_secret() else {
        return Err(AuthApiError::LocalSessionUnavailable);
    };
    // 比较双方的 SHA-256 摘要而非明文，避免逐字节短路比较造成时序泄露。
    if Sha256::digest(request.exchange_token.as_bytes())
        != Sha256::digest(secret.expose().as_bytes())
    {
        return Err(AuthApiError::InvalidCredentials);
    }

    let user = resolve_local_auto_login_user(state.database()).await?;
    issue_session_response(
        state,
        &user,
        request.device_name,
        request.client_kind,
        request.version,
    )
    .await
}

/// 返回本机静默会话状态；仅当当前用户就是标记用户且密码仍为占位时为 true。
pub(crate) async fn local_session_status(
    state: &CoreState,
    current_user: &CurrentUser,
) -> Result<AuthLocalSessionStatus, AuthApiError> {
    Ok(AuthLocalSessionStatus {
        password_placeholder: password_placeholder_active(state.database(), current_user.user_id)
            .await?,
    })
}

/// 为已通过身份确认的用户签发 access/refresh token 对；登录与本机换取共用本路径。
async fn issue_session_response(
    state: &CoreState,
    user: &user::Model,
    device_name: String,
    client_kind: AuthClientKind,
    version: String,
) -> Result<AuthTokenResponse, AuthApiError> {
    let rbac = RbacRepository::new(state.database());
    let user_response = load_user_response(&rbac, user).await?;
    let access_token = state
        .security()
        .issue_access_token(user.id, user_response.permissions.clone())?;
    let refresh_token =
        create_refresh_token(state, user.id, device_name, client_kind, version).await?;

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
    let access_token = state
        .security()
        .issue_access_token(user.id, user_response.permissions.clone())?;

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
