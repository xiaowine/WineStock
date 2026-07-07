//! core 全局认证与授权前置层入口。
//!
//! 本模块属于 `core axum library` 层，负责 bearer token 解析、当前用户提取、
//! JWT 支持、密码与令牌安全工具，以及路由级权限 middleware。
//! 它不实现登录、注册、登出或当前用户响应组装等具体业务流程。

mod current_user;
mod error;
mod jwt;
mod middleware;
mod password;
mod token;

pub use current_user::CurrentUser;
pub use error::AuthApiError;

#[cfg(test)]
pub(crate) use jwt::AccessClaims;
pub(crate) use jwt::SecurityRuntime;
#[cfg(test)]
pub(crate) use middleware::require_permission;
pub(crate) use middleware::{users_exist, AuthorizeRouteExt};
pub(crate) use password::{create_password_hash, verify_password};
#[cfg(test)]
pub(crate) use token::unix_timestamp;
pub(crate) use token::{hash_refresh_token, random_urlsafe};

#[cfg(test)]
#[path = "../tests/security_authorization.rs"]
mod security_authorization_tests;
