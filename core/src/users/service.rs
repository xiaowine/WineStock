//! users 模块业务服务。
//!
//! 本模块属于 `users` 业务层，负责注册、当前用户查询以及用户响应组装。
//! 它不负责路由级鉴权条件，也不直接解析 bearer token。

use winestock_shared::{AuthRegisterRequest, AuthUserResponse};

use crate::{
    persistence::{
        entity::user,
        repository::{AuthRepository, CreateUser, RbacRepository, UserRepository},
    },
    rbac::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME},
    security::{create_password_hash, AuthApiError, CurrentUser},
    state::CoreState,
};

/// 执行注册用例；当数据库尚无用户时，首个用户会被分配为 admin。
pub(crate) async fn register(
    state: &CoreState,
    request: AuthRegisterRequest,
) -> Result<AuthUserResponse, AuthApiError> {
    let username = normalize_username(&request.username)?;
    if request.password.is_empty() {
        return Err(AuthApiError::InvalidRegisterRequest);
    }

    let auth_repository = AuthRepository::new(state.database());
    let users = UserRepository::new(state.database());
    let rbac = RbacRepository::new(state.database());
    let has_users = auth_repository.has_any_user().await?;

    if users.find_by_username(&username).await?.is_some() {
        return Err(AuthApiError::UsernameTaken);
    }

    let user = users
        .create_user(CreateUser {
            username,
            password_hash: create_password_hash(&request.password)?,
            display_name: None,
        })
        .await?;

    if !has_users {
        let admin_role_id = rbac
            .ensure_role(
                ADMIN_ROLE_CODE,
                ADMIN_ROLE_NAME,
                Some("系统管理员，拥有全部内置权限。"),
            )
            .await?;
        rbac.assign_role_to_user(user.id, admin_role_id).await?;
    }

    load_user_response(&rbac, &user).await
}

/// 根据当前认证上下文读取数据库中的最新用户、角色和权限快照。
pub(crate) async fn current_user(
    state: &CoreState,
    current_user: &CurrentUser,
) -> Result<AuthUserResponse, AuthApiError> {
    let users = UserRepository::new(state.database());
    let Some(user) = users.find_by_id(current_user.user_id).await? else {
        return Err(AuthApiError::InvalidAccessToken);
    };
    if user.status != "active" {
        return Err(AuthApiError::InvalidAccessToken);
    }

    let rbac = RbacRepository::new(state.database());
    load_user_response(&rbac, &user).await
}

/// 组装 API 返回和 JWT claims 共享的用户、角色、权限快照。
pub(crate) async fn load_user_response(
    rbac: &RbacRepository<'_>,
    user: &user::Model,
) -> Result<AuthUserResponse, AuthApiError> {
    Ok(AuthUserResponse {
        id: user.id.to_string(),
        username: user.username.clone(),
        roles: rbac.list_user_roles(user.id).await?,
        permissions: rbac.list_user_permissions(user.id).await?,
    })
}

/// 规范化用户名，避免空白用户名或仅靠首尾空白区分账号。
fn normalize_username(username: &str) -> Result<String, AuthApiError> {
    let username = username.trim();
    if username.is_empty() {
        Err(AuthApiError::InvalidRegisterRequest)
    } else {
        Ok(username.to_owned())
    }
}
