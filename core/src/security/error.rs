//! 鉴权相关 HTTP 错误定义。
//!
//! 本模块属于 `security` 前置层，为 `security`、`auth` 和 `users` 模块共享统一的
//! HTTP 错误映射。它不执行数据库查询，也不承载具体业务流程。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

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

    /// 用户名或密码错误，响应不暴露具体失败点。
    InvalidCredentials,

    /// refresh token 不存在、过期、已吊销或复用。
    InvalidRefreshToken,

    /// access token 缺失、格式错误、过期或签名无效。
    InvalidAccessToken,

    /// 当前用户缺少访问资源所需权限。
    PermissionDenied,

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
        match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::InvalidRegisterRequest => (StatusCode::BAD_REQUEST, "invalid_register_request"),
            Self::UsernameTaken => (StatusCode::CONFLICT, "username_taken"),
            Self::UserNotFound => (StatusCode::NOT_FOUND, "user_not_found"),
            Self::PermissionNotFound => (StatusCode::NOT_FOUND, "permission_not_found"),
            Self::LastPermissionManagerRequired => {
                (StatusCode::CONFLICT, "last_permission_manager_required")
            }
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            Self::InvalidRefreshToken => (StatusCode::UNAUTHORIZED, "invalid_refresh_token"),
            Self::InvalidAccessToken => (StatusCode::UNAUTHORIZED, "invalid_access_token"),
            Self::PermissionDenied => (StatusCode::FORBIDDEN, "permission_denied"),
            Self::Database(_) | Self::Jwt(_) | Self::Random(_) | Self::Internal => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_auth_error")
            }
        }
        .into_response()
    }
}

impl From<DbErr> for AuthApiError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}
