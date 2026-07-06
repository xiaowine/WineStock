//! 鉴权和权限 middleware。
//!
//! 本模块属于 core 鉴权层，负责在 Axum route layer 中完成 Bearer token 校验、
//! 当前权限读取和权限判断。业务 handler 不应直接判断角色或权限代码。

use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::MethodRouter,
};
use std::{future::Future, pin::Pin, sync::Arc};

use crate::persistence::repository::{AuthRepository, RbacRepository, UserRepository};

use super::{
    error::AuthApiError,
    runtime::{bearer_token_from_headers, AuthRuntime, CurrentUser},
};

/// 条件鉴权的异步判断函数；返回 true 表示本次请求需要执行权限校验。
#[derive(Clone)]
pub(crate) struct AuthorizationCondition {
    evaluate: Arc<
        dyn Fn(AuthRuntime) -> Pin<Box<dyn Future<Output = Result<bool, AuthApiError>> + Send>>
            + Send
            + Sync,
    >,
}

impl AuthorizationCondition {
    /// 创建可复用的条件鉴权判断，供路由注册时按业务条件组合权限要求。
    pub(crate) fn new<F, Fut>(evaluate: F) -> Self
    where
        F: Fn(AuthRuntime) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<bool, AuthApiError>> + Send + 'static,
    {
        Self {
            evaluate: Arc::new(move |auth_state| Box::pin(evaluate(auth_state))),
        }
    }

    async fn should_enforce(&self, auth_state: &AuthRuntime) -> Result<bool, AuthApiError> {
        (self.evaluate)(auth_state.clone()).await
    }
}

/// 给路由增加“必须登录”的鉴权层，并把当前用户写入 request extensions。
pub(crate) fn require_authenticated(
    route: MethodRouter<AuthRuntime>,
    auth_state: AuthRuntime,
) -> MethodRouter<AuthRuntime> {
    apply_authorization(route, auth_state, AuthorizationPolicy::Authenticated)
}

/// 给路由增加指定权限校验；适合后续普通业务 API 直接声明权限代码。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn require_permission(
    route: MethodRouter<AuthRuntime>,
    auth_state: AuthRuntime,
    permission: &'static str,
) -> MethodRouter<AuthRuntime> {
    apply_authorization(
        route,
        auth_state,
        AuthorizationPolicy::Permission(permission),
    )
}

/// 给路由增加条件权限校验；条件满足时才要求调用方拥有指定权限。
pub(crate) fn require_permission_when(
    route: MethodRouter<AuthRuntime>,
    auth_state: AuthRuntime,
    permission: &'static str,
    condition: AuthorizationCondition,
) -> MethodRouter<AuthRuntime> {
    apply_authorization(
        route,
        auth_state,
        AuthorizationPolicy::ConditionalPermission {
            permission,
            condition,
        },
    )
}

/// 创建“数据库已有用户”条件；注册接口用它表达首个用户免鉴权。
pub(crate) fn users_exist() -> AuthorizationCondition {
    AuthorizationCondition::new(|auth_state| async move {
        AuthRepository::new(&auth_state.database)
            .has_any_user()
            .await
            .map_err(AuthApiError::Database)
    })
}

fn apply_authorization(
    route: MethodRouter<AuthRuntime>,
    auth_state: AuthRuntime,
    policy: AuthorizationPolicy,
) -> MethodRouter<AuthRuntime> {
    route.route_layer(middleware::from_fn_with_state(
        AuthorizationState { auth_state, policy },
        authorize,
    ))
}

#[derive(Clone)]
struct AuthorizationState {
    auth_state: AuthRuntime,
    policy: AuthorizationPolicy,
}

#[derive(Clone)]
enum AuthorizationPolicy {
    Authenticated,
    #[cfg_attr(not(test), allow(dead_code))]
    Permission(&'static str),
    ConditionalPermission {
        permission: &'static str,
        condition: AuthorizationCondition,
    },
}

/// 在进入业务 handler 前统一完成鉴权；成功后把数据库当前授权快照放入请求扩展。
async fn authorize(
    State(state): State<AuthorizationState>,
    mut request: Request,
    next: Next,
) -> Response {
    match resolve_requirement(&state).await {
        Ok(AuthorizationRequirement::Bypass) => next.run(request).await,
        Ok(requirement) => {
            let permission = match requirement {
                AuthorizationRequirement::Bypass => unreachable!("Bypass 已在上方提前返回"),
                AuthorizationRequirement::Authenticated => None,
                AuthorizationRequirement::Permission(permission) => Some(permission),
            };
            let token = match bearer_token_from_headers(request.headers()) {
                Some(token) => token.to_owned(),
                None => return AuthApiError::InvalidAccessToken.into_response(),
            };
            let current_user = match load_current_user_from_token(&state.auth_state, &token).await {
                Ok(current_user) => current_user,
                Err(error) => return error.into_response(),
            };
            if let Some(permission) = permission {
                if !current_user.has_permission(permission) {
                    return AuthApiError::PermissionDenied.into_response();
                }
            }

            request.extensions_mut().insert(current_user);
            next.run(request).await
        }
        Err(error) => error.into_response(),
    }
}

async fn resolve_requirement(
    state: &AuthorizationState,
) -> Result<AuthorizationRequirement, AuthApiError> {
    match state.policy {
        AuthorizationPolicy::Authenticated => Ok(AuthorizationRequirement::Authenticated),
        AuthorizationPolicy::Permission(permission) => {
            Ok(AuthorizationRequirement::Permission(permission))
        }
        AuthorizationPolicy::ConditionalPermission {
            permission,
            ref condition,
        } => {
            if !condition.should_enforce(&state.auth_state).await? {
                return Ok(AuthorizationRequirement::Bypass);
            }

            Ok(AuthorizationRequirement::Permission(permission))
        }
    }
}

enum AuthorizationRequirement {
    Bypass,
    Authenticated,
    Permission(&'static str),
}

/// 从 Bearer token 得到当前用户，并重新读取数据库中的当前角色和权限。
async fn load_current_user_from_token(
    auth_state: &AuthRuntime,
    token: &str,
) -> Result<CurrentUser, AuthApiError> {
    let mut current_user = auth_state.verify_access_token(token)?;
    let users = UserRepository::new(&auth_state.database);
    let Some(user) = users.find_by_id(current_user.user_id).await? else {
        return Err(AuthApiError::InvalidAccessToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidAccessToken);
    }

    let rbac = RbacRepository::new(&auth_state.database);
    current_user.roles = rbac.list_user_roles(current_user.user_id).await?;
    current_user.permissions = rbac.list_user_permissions(current_user.user_id).await?;

    Ok(current_user)
}
