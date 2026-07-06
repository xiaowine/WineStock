//! 鉴权 HTTP 错误。
//!
//! 本模块属于 core 鉴权层，集中把鉴权、token 和数据库错误映射成 HTTP 响应。
//! 它不执行数据库查询，也不暴露内部错误细节给客户端。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

/// 鉴权 HTTP API 的错误响应。
#[derive(Debug)]
pub enum AuthApiError {
    /// 注册请求字段不满足服务端约束。
    InvalidRegisterRequest,

    /// 用户名已经存在。
    UsernameTaken,

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
            Self::InvalidRegisterRequest => (StatusCode::BAD_REQUEST, "invalid_register_request"),
            Self::UsernameTaken => (StatusCode::CONFLICT, "username_taken"),
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
