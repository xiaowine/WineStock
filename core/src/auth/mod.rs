//! auth 会话认证业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责登录、刷新、登出和鉴权启动初始化。
//! 它依赖 `security` 提供 JWT 与令牌工具，但自身不是全局安全前置层。

use axum::{
    routing::post,
    Router,
};

use crate::state::CoreState;

mod bootstrap;
pub(crate) mod controller;
pub(crate) mod service;

pub use bootstrap::{
    AuthBootstrap, AuthBootstrapError, AuthSettings, AuthSigningKey, SigningKeyStatus,
};

pub(crate) use bootstrap::bootstrap_auth;

/// 注册会话认证业务自己的 HTTP 路由集合。
pub(crate) fn router() -> Router<CoreState> {
    Router::new()
        .route("/api/auth/login", post(controller::login))
        .route("/api/auth/refresh", post(controller::refresh))
        .route("/api/auth/logout", post(controller::logout))
}

#[cfg(test)]
#[path = "../tests/auth_login.rs"]
mod auth_login_tests;

#[cfg(test)]
#[path = "../tests/auth_refresh.rs"]
mod auth_refresh_tests;

#[cfg(test)]
#[path = "../tests/auth_logout.rs"]
mod auth_logout_tests;
