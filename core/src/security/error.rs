//! 鉴权相关 HTTP 错误定义。
//!
//! 本模块属于 `security` 前置层，为 `security`、`auth` 和 `users` 模块共享统一的
//! HTTP 错误映射。它不执行数据库查询，也不承载具体业务流程。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

use crate::http::api_error_response;

/// 鉴权相关 API 的错误响应。
#[derive(Debug)]
pub enum AuthApiError {
    /// 请求字段通过 JSON 解析但不满足业务约束。
    InvalidRequest,

    /// 注册请求字段不满足服务端约束。
    InvalidRegisterRequest,

    /// 用户名已经存在。
    UsernameTaken,

    /// 指定用户不存在。
    UserNotFound,

    /// 指定权限不存在。
    PermissionNotFound,

    /// 操作会导致系统没有可分配权限的 active 用户。
    LastPermissionManagerRequired,

    /// 当前操作者试图停用自己的账号。
    SelfStatusUpdateForbidden,

    /// 当前操作者试图软删除自己的账号。
    SelfDeleteForbidden,

    /// 当前操作者试图为自己的账号设置管理员临时密码。
    SelfPasswordResetForbidden,

    /// 当前操作者试图改动自己账号上的受保护权限。
    SelfProtectedPermissionsUpdateForbidden,

    /// 用户名或密码错误，响应不暴露具体失败点。
    InvalidCredentials,

    /// refresh token 不存在、过期、已吊销或复用。
    InvalidRefreshToken,

    /// access token 缺失、格式错误、过期或签名无效。
    InvalidAccessToken,

    /// 当前用户缺少访问资源所需权限。
    PermissionDenied,

    /// 当前用户必须先修改临时密码。
    PasswordChangeRequired,

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
        // 对外固定 JSON 错误契约，内部异常只收敛为稳定错误码。
        let (status, code, message) = match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request", "请求参数无效"),
            Self::InvalidRegisterRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_register_request",
                "注册请求无效",
            ),
            Self::UsernameTaken => (StatusCode::CONFLICT, "username_taken", "用户名已存在"),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "user_not_found", "用户不存在"),
            Self::PermissionNotFound => {
                (StatusCode::NOT_FOUND, "permission_not_found", "权限不存在")
            }
            Self::LastPermissionManagerRequired => (
                StatusCode::CONFLICT,
                "last_permission_manager_required",
                "至少需要保留一个可管理权限的启用用户",
            ),
            Self::SelfStatusUpdateForbidden => (
                StatusCode::FORBIDDEN,
                "self_status_update_forbidden",
                "不能停用当前账号",
            ),
            Self::SelfDeleteForbidden => (
                StatusCode::FORBIDDEN,
                "self_user_delete_forbidden",
                "不能删除当前账号",
            ),
            Self::SelfPasswordResetForbidden => (
                StatusCode::FORBIDDEN,
                "self_password_reset_forbidden",
                "不能为当前账号设置临时密码",
            ),
            Self::SelfProtectedPermissionsUpdateForbidden => (
                StatusCode::FORBIDDEN,
                "self_protected_permissions_update_forbidden",
                "不能修改当前账号的关键权限",
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "用户名或密码错误",
            ),
            Self::InvalidRefreshToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_refresh_token",
                "刷新令牌无效",
            ),
            Self::InvalidAccessToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_access_token",
                "访问令牌无效",
            ),
            Self::PermissionDenied => (StatusCode::FORBIDDEN, "permission_denied", "没有操作权限"),
            Self::PasswordChangeRequired => (
                StatusCode::FORBIDDEN,
                "password_change_required",
                "需要先修改临时密码",
            ),
            Self::Database(_) | Self::Jwt(_) | Self::Random(_) | Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_auth_error",
                "鉴权服务内部错误",
            ),
        };

        api_error_response(status, code, message)
    }
}

impl From<DbErr> for AuthApiError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}
