//! users 模块 HTTP 控制器。
//!
//! 本模块属于 `users` 业务层，负责把用户业务相关 HTTP 请求转发到服务实现。
//! URL 仍保持 `/api/auth/*` 兼容路径，但具体实现已收敛到用户业务模块。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use winestock_shared::{AuthRegisterRequest, AuthUserResponse};

use crate::{
    http::ValidatedJson,
    security::{AuthApiError, CurrentUser},
    state::CoreState,
};

use super::service;

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
/// 注册新用户；首个用户免鉴权并自动成为 admin，之后必须拥有注册用户权限。
pub(crate) async fn register(
    State(state): State<CoreState>,
    current_user: Option<Extension<CurrentUser>>,
    ValidatedJson(request): ValidatedJson<AuthRegisterRequest>,
) -> Result<(StatusCode, Json<AuthUserResponse>), AuthApiError> {
    let current_user = current_user.map(|Extension(user)| user);
    Ok((
        StatusCode::CREATED,
        Json(service::register(&state, request, current_user.as_ref()).await?),
    ))
}

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
/// 返回 bearer token 对应的当前用户。
pub(crate) async fn me(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<AuthUserResponse>, AuthApiError> {
    Ok(Json(service::current_user(&state, &current_user).await?))
}
