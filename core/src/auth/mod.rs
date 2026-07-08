//! auth 会话认证业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责登录、刷新、登出和鉴权启动初始化。
//! 它依赖 `security` 提供 JWT 与令牌工具，但自身不是全局安全前置层。

use axum::{routing::post, Router};

use crate::state::CoreState;

mod bootstrap;
pub(crate) mod controller;
pub(crate) mod service;

pub use bootstrap::{
    AuthBootstrap, AuthBootstrapError, AuthSettings, AuthSigningKey, SigningKeyStatus,
};

pub(crate) use bootstrap::bootstrap_auth;

const AUTH_BASE_PATH: &str = "/api/auth";

/// 注册会话认证业务自己的 HTTP 路由集合。
pub(crate) fn router() -> Router<CoreState> {
    // 会话认证接口统一挂载在固定 base path，子路由只声明领域内相对路径。
    Router::new().nest(
        AUTH_BASE_PATH,
        Router::new()
            .route("/login", post(controller::login))
            .route("/refresh", post(controller::refresh))
            .route("/logout", post(controller::logout)),
    )
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
