//! core 的鉴权模块入口。
//!
//! 本模块属于 `core axum library` 层，组合鉴权启动、JWT 运行时、HTTP handler
//! 和安全工具。具体职责下沉到子模块，避免一个文件同时拥有所有鉴权细节。
//! 它不拥有平台交互流程，也不把签名密钥或 refresh token 明文暴露给平台配置。

mod authorization;
mod bootstrap;
mod error;
pub(crate) mod routes;
mod runtime;
mod security;

#[cfg(test)]
#[path = "../tests/auth.rs"]
mod tests;

pub use bootstrap::{
    AuthBootstrap, AuthBootstrapError, AuthSettings, AuthSigningKey, SigningKeyStatus,
};
pub use error::AuthApiError;
pub use runtime::CurrentUser;

#[allow(unused_imports)]
pub(crate) use authorization::require_permission;
pub(crate) use authorization::{require_authenticated, require_permission_when, users_exist};
pub(crate) use bootstrap::bootstrap_auth;
pub(crate) use routes::{login, logout, me, refresh, register};
pub(crate) use runtime::AuthRuntime;

pub(crate) const REGISTER_USER_PERMISSION: &str = "user.register";
