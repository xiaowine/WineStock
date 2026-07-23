//! auth 模块 HTTP 控制器。
//!
//! 本模块属于 `auth` 业务层，负责把会话认证相关 HTTP 请求转发到服务实现。
//! 它不直接拼接数据库查询，也不自行维护令牌状态。

use axum::{extract::State, http::StatusCode, Json};

use crate::{
    auth::{AuthBootstrapStatus, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthTokenResponse},
    persistence::repository::AuthRepository,
    http::ValidatedJson,
    security::AuthApiError,
    state::CoreState,
};

use super::service;

#[utoipa::path(
    get,
    path = "/api/auth/bootstrap-status",
    tag = "auth",
    responses(
        (status = 200, description = "Authentication bootstrap status", body = AuthBootstrapStatus),
        (status = 500, description = "Authentication service error", body = crate::http::ApiErrorResponse)
    )
)]
/// 返回认证入口是否需要创建服务的首个用户；不返回用户数量或用户资料。
pub(crate) async fn bootstrap_status(
    State(state): State<CoreState>,
) -> Result<Json<AuthBootstrapStatus>, AuthApiError> {
    let requires_initial_user = !AuthRepository::new(state.database()).has_any_user().await?;
    Ok(Json(AuthBootstrapStatus { requires_initial_user }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = AuthLoginRequest,
    responses(
        (status = 200, description = "Login succeeded", body = AuthTokenResponse),
        (status = 401, description = "Invalid credentials", body = crate::http::ApiErrorResponse)
    )
)]
/// 用户名密码登录，成功后返回 JWT access token 和 opaque refresh token。
pub(crate) async fn login(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<AuthLoginRequest>,
) -> Result<Json<AuthTokenResponse>, AuthApiError> {
    Ok(Json(service::login(&state, request).await?))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = AuthRefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = AuthTokenResponse),
        (status = 401, description = "Invalid refresh token", body = crate::http::ApiErrorResponse)
    )
)]
/// 使用 refresh token 轮换并签发新的 access token。
pub(crate) async fn refresh(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<AuthRefreshRequest>,
) -> Result<Json<AuthTokenResponse>, AuthApiError> {
    Ok(Json(service::refresh(&state, request).await?))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    request_body = AuthLogoutRequest,
    responses(
        (status = 204, description = "Logged out"),
        (status = 401, description = "Invalid refresh token", body = crate::http::ApiErrorResponse)
    )
)]
/// 吊销当前 refresh token；access token 自身仍按短 TTL 自然过期。
pub(crate) async fn logout(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<AuthLogoutRequest>,
) -> Result<StatusCode, AuthApiError> {
    service::logout(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}
