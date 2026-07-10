//! users 模块 HTTP 控制器。
//!
//! 本模块属于 `users` 业务层，负责把用户业务相关 HTTP 请求转发到服务实现。
//! 注册和当前用户相关 URL 仍保持 `/api/auth/*` 兼容路径，管理接口使用 `/api/users` 和权限只读路径。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthRegisterRequest, AuthUserResponse},
    http::{ValidatedJson, ValidatedPath, ValidatedQuery},
    security::{AuthApiError, CurrentUser},
    state::CoreState,
    validation::{validate_code_list, validate_not_blank},
};

use super::service::{self, PaginatedResponse};

/// 用户状态。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UserStatus {
    /// 可登录和访问 API 的有效账号。
    Active,

    /// 已停用账号；现有 access token 和 refresh token 都会被拒绝。
    Disabled,
}

impl UserStatus {
    /// 返回数据库中保存的稳定状态代码。
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }

    /// 从数据库状态代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, AuthApiError> {
        match value {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            _ => Err(AuthApiError::InvalidRequest),
        }
    }
}

/// 用户列表查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct UserListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按用户名模糊搜索。
    pub search: Option<String>,

    /// 按用户状态筛选，允许 `active` 或 `disabled`。
    pub status: Option<String>,
}

/// 用户管理响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct UserAdminResponse {
    /// 用户 ID。
    pub id: i64,

    /// 登录用户名。
    pub username: String,

    /// 用户状态。
    pub status: UserStatus,

    /// 用户直接拥有的权限代码。
    pub permissions: Vec<String>,

    /// 是否必须先修改密码；管理员设置临时密码后该值为 true。
    pub password_change_required: bool,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 用户状态更新请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserStatusUpdateRequest {
    /// 新用户状态。
    #[garde(dive)]
    pub status: UserStatus,
}

/// 用户权限整体替换请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserPermissionsUpdateRequest {
    /// 权限代码列表；空列表表示清空该用户权限。
    #[garde(length(max = 32), custom(validate_code_list))]
    pub permissions: Vec<String>,
}

/// 管理员设置临时密码请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserPasswordResetRequest {
    /// 临时明文密码，只允许出现在本请求中，服务端只保存 Argon2 哈希。
    #[garde(length(min = 8, max = 128), custom(validate_not_blank))]
    pub password: String,
}

/// 当前用户修改自己密码的请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct UserPasswordChangeRequest {
    /// 当前明文密码，用于确认操作者仍掌握原凭据。
    #[garde(length(min = 1, max = 256), custom(validate_not_blank))]
    pub current_password: String,

    /// 新明文密码，只允许出现在本请求中，服务端只保存 Argon2 哈希。
    #[garde(length(min = 8, max = 128), custom(validate_not_blank))]
    pub new_password: String,
}

/// 权限响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PermissionResponse {
    /// 稳定权限代码。
    pub code: String,

    /// 权限说明。
    pub description: Option<String>,
}

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
        (status = 400, description = "Invalid register request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Register permission required", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Username already exists", body = crate::http::ApiErrorResponse)
    )
)]
/// 注册新用户；首个用户免鉴权并自动获得全部内置权限，之后必须拥有注册用户权限。
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
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Permission denied", body = crate::http::ApiErrorResponse)
    )
)]
/// 返回 bearer token 对应的当前用户。
pub(crate) async fn me(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<AuthUserResponse>, AuthApiError> {
    Ok(Json(service::current_user(&state, &current_user).await?))
}

#[utoipa::path(
    post,
    path = "/api/auth/me/password",
    tag = "auth",
    request_body = UserPasswordChangeRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Password changed"),
        (status = 400, description = "Invalid request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token or current password", body = crate::http::ApiErrorResponse)
    )
)]
/// 当前登录用户修改自己的密码；该接口不允许指定其他用户 ID。
pub(crate) async fn change_own_password(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<UserPasswordChangeRequest>,
) -> Result<StatusCode, AuthApiError> {
    service::change_own_password(&state, &current_user, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    params(UserListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User list", body = PaginatedResponse<UserAdminResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "User manage permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询用户管理列表。
pub(crate) async fn list_users(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<UserListQuery>,
) -> Result<Json<PaginatedResponse<UserAdminResponse>>, AuthApiError> {
    Ok(Json(service::list_users(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "users",
    params(("id" = i64, Path, description = "User ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User detail", body = UserAdminResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "User manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "User not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询单个用户管理详情。
pub(crate) async fn get_user(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<UserAdminResponse>, AuthApiError> {
    Ok(Json(service::get_user(&state, id).await?))
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}/status",
    tag = "users",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UserStatusUpdateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User status updated", body = UserAdminResponse),
        (status = 400, description = "Invalid request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "User manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "User not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Last active permission manager cannot be disabled", body = crate::http::ApiErrorResponse)
    )
)]
/// 更新用户状态。
pub(crate) async fn update_user_status(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<UserStatusUpdateRequest>,
) -> Result<Json<UserAdminResponse>, AuthApiError> {
    Ok(Json(
        service::update_user_status(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/users/{id}/permissions",
    tag = "users",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UserPermissionsUpdateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User permissions updated", body = UserAdminResponse),
        (status = 400, description = "Invalid request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "User manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "User or permission not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Last active permission manager cannot lose manage permission", body = crate::http::ApiErrorResponse)
    )
)]
/// 整体替换用户权限。
pub(crate) async fn update_user_permissions(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<UserPermissionsUpdateRequest>,
) -> Result<Json<UserAdminResponse>, AuthApiError> {
    Ok(Json(
        service::update_user_permissions(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/users/{id}/password",
    tag = "users",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UserPasswordResetRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Temporary password set"),
        (status = 400, description = "Invalid request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Password reset permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "User not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 拥有重置密码权限的用户设置目标用户临时密码，目标用户下次登录后必须改密。
pub(crate) async fn reset_user_password(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<UserPasswordResetRequest>,
) -> Result<StatusCode, AuthApiError> {
    service::reset_user_password(&state, &current_user, id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/permissions",
    tag = "users",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Permission list", body = Vec<PermissionResponse>),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "User manage permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询权限定义列表。
pub(crate) async fn list_permissions(
    State(state): State<CoreState>,
) -> Result<Json<Vec<PermissionResponse>>, AuthApiError> {
    Ok(Json(service::list_permissions(&state).await?))
}
