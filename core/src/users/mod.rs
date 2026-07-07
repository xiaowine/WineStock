//! users 用户业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责注册、当前用户和后续用户管理能力。
//! 它依赖 `security` 提供当前用户上下文和权限校验，但不直接承担 token 解析。

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    security::{users_exist, AuthorizeRouteExt},
    state::CoreState,
};

pub(crate) mod controller;
mod permissions;
pub(crate) mod service;

pub(crate) use permissions::{MANAGE_USER_PERMISSION, REGISTER_USER_PERMISSION};

/// 注册用户业务自己的 HTTP 路由集合。
pub(crate) fn router(state: CoreState) -> Router<CoreState> {
    Router::new()
        .route(
            "/api/auth/register",
            post(controller::register).require_permission_when(
                state.clone(),
                REGISTER_USER_PERMISSION,
                users_exist(),
            ),
        )
        .route(
            "/api/auth/me",
            get(controller::me).require_authenticated(state.clone()),
        )
}

#[cfg(test)]
#[path = "../tests/users_register.rs"]
mod users_register_tests;

#[cfg(test)]
#[path = "../tests/users_me.rs"]
mod users_me_tests;
