//! 鉴权 HTTP 处理函数。
//!
//! 本模块属于 core 鉴权层，拥有注册、登录、刷新、登出和当前用户接口。
//! 它组合 repository、运行时 token 能力和 RBAC 查询，不直接拥有平台生命周期。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use winestock_shared::{
    AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};

use crate::{
    persistence::{
        entity::{refresh_token, user},
        repository::{
            sqlite_now, sqlite_time_after_seconds, AuthRepository, CreateRefreshToken, CreateUser,
            RbacRepository, RefreshTokenRepository, UserRepository,
        },
    },
    rbac::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME},
};

use super::{
    error::AuthApiError,
    runtime::{AuthRuntime, CurrentUser},
    security::{create_password_hash, hash_refresh_token, random_urlsafe, verify_password},
};

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
    Extension(current_user): Extension<CurrentUser>,
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

/// 规范化用户名，避免空白用户名或仅靠首尾空白区分账号。
fn normalize_username(username: &str) -> Result<String, AuthApiError> {
    let username = username.trim();
    if username.is_empty() {
        Err(AuthApiError::InvalidRegisterRequest)
    } else {
        Ok(username.to_owned())
    }
}
