//! auth 会话认证业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责登录、刷新、登出和鉴权启动初始化。
//! 它依赖 `security` 提供 JWT 与令牌工具，但自身不是全局安全前置层。

use axum::{
    routing::{get, post},
    Router,
};

use crate::{security::AuthorizeRouteExt, state::CoreState};

mod bootstrap;
mod contract;
pub(crate) mod controller;
pub(crate) mod service;

pub use bootstrap::{
    AuthBootstrap, AuthBootstrapError, AuthSettings, AuthSigningKey, SigningKeyStatus,
};
pub use contract::{
    AuthBootstrapStatus, AuthClientKind, AuthLocalSessionRequest, AuthLocalSessionStatus,
    AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};

pub(crate) use bootstrap::bootstrap_auth;

const AUTH_BASE_PATH: &str = "/api/auth";

/// 注册会话认证业务自己的 HTTP 路由集合。
pub(crate) fn router(state: CoreState) -> Router<CoreState> {
    // 会话认证接口统一挂载在固定 base path，子路由只声明领域内相对路径。
    // local-session 换取本身匿名（凭据校验在 handler 内），状态查询要求已登录。
    Router::new().nest(
        AUTH_BASE_PATH,
        Router::new()
            .route("/bootstrap-status", get(controller::bootstrap_status))
            .route("/login", post(controller::login))
            .route("/local-session", post(controller::local_session))
            .route(
                "/local-session/status",
                get(controller::local_session_status).require_authenticated(state),
            )
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

#[cfg(test)]
#[path = "../tests/auth_local_session.rs"]
mod auth_local_session_tests;
