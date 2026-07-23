//! users 用户业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责注册、当前用户、自助改密码和用户管理能力。
//! 它依赖 `security` 提供当前用户上下文和权限校验，但不直接承担 token 解析。

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

use crate::{
    security::{users_exist, AuthApiError, AuthorizeRouteExt},
    state::CoreState,
};

pub(crate) mod controller;
mod permissions;
pub(crate) mod service;

pub(crate) use permissions::{
    DELETE_USER_PERMISSION, READ_USER_PERMISSION, READ_USER_PERMISSION_DEFINITION_PERMISSION,
    REGISTER_USER_PERMISSION, RESET_USER_PASSWORD_PERMISSION, UPDATE_USER_PERMISSIONS_PERMISSION,
    UPDATE_USER_STATUS_PERMISSION,
};

/// 注册用户业务 HTTP 路由集合。
pub(crate) fn router(state: CoreState) -> Router<CoreState> {
    Router::new()
        .route(
            "/api/auth/register",
            post(controller::register).require_permission_when_with_anonymous_error(
                state.clone(),
                REGISTER_USER_PERMISSION,
                users_exist(),
                AuthApiError::InitialUserAlreadyExists,
            ),
        )
        .route(
            "/api/auth/me",
            get(controller::me).require_authenticated(state.clone()),
        )
        .route(
            "/api/auth/me/password",
            post(controller::change_own_password).require_authenticated(state.clone()),
        )
        .route(
            "/api/users",
            get(controller::list_users).require_permission(state.clone(), READ_USER_PERMISSION),
        )
        .route(
            "/api/users/{id}",
            get(controller::get_user)
                .require_permission(state.clone(), READ_USER_PERMISSION)
                .merge(
                    delete(controller::delete_user)
                        .require_permission(state.clone(), DELETE_USER_PERMISSION),
                ),
        )
        .route(
            "/api/users/{id}/status",
            patch(controller::update_user_status)
                .require_permission(state.clone(), UPDATE_USER_STATUS_PERMISSION),
        )
        .route(
            "/api/users/{id}/permissions",
            put(controller::update_user_permissions)
                .require_permission(state.clone(), UPDATE_USER_PERMISSIONS_PERMISSION),
        )
        .route(
            "/api/users/{id}/password",
            post(controller::reset_user_password)
                .require_permission(state.clone(), RESET_USER_PASSWORD_PERMISSION),
        )
        .route(
            "/api/permissions",
            get(controller::list_permissions)
                .require_permission(state.clone(), READ_USER_PERMISSION_DEFINITION_PERMISSION),
        )
}

#[cfg(test)]
#[path = "../tests/users_register.rs"]
mod users_register_tests;

#[cfg(test)]
#[path = "../tests/users_me.rs"]
mod users_me_tests;

#[cfg(test)]
#[path = "../tests/users_management.rs"]
mod users_management_tests;
