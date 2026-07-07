//! users 模块业务服务。
//!
//! 本模块属于 `users` 业务层，负责注册、当前用户查询以及用户响应组装。
//! 它不负责路由级鉴权条件，也不直接解析 bearer token。

use winestock_shared::{AuthRegisterRequest, AuthUserResponse};

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};

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
    current_user: Option<&CurrentUser>,
) -> Result<AuthUserResponse, AuthApiError> {
    let username = normalize_username(&request.username)?;
    if request.password.is_empty() {
        return Err(AuthApiError::InvalidRegisterRequest);
    }
    let password_hash = create_password_hash(&request.password)?;

    let user = register_user_transactionally(
        state.database(),
        CreateUser {
            username,
            password_hash,
            display_name: None,
        },
        current_user,
    )
    .await?;

    let rbac = RbacRepository::new(state.database());
    load_user_response(&rbac, &user).await
}

/// 在同一事务中完成注册、首个用户判断和首个 admin 分配，避免并发首登产生多个管理员。
async fn register_user_transactionally(
    database: &DatabaseConnection,
    input: CreateUser,
    current_user: Option<&CurrentUser>,
) -> Result<user::Model, AuthApiError> {
    let transaction = database.begin().await?;
    acquire_registration_write_lock(&transaction).await?;

    let auth_repository = AuthRepository::new(&transaction);
    let users = UserRepository::new(&transaction);
    let rbac = RbacRepository::new(&transaction);
    let has_users = auth_repository.has_any_user().await?;

    if has_users {
        let Some(current_user) = current_user else {
            return Err(AuthApiError::InvalidAccessToken);
        };
        if !current_user.has_permission(super::REGISTER_USER_PERMISSION) {
            return Err(AuthApiError::PermissionDenied);
        }
    }

    if users.find_by_username(&input.username).await?.is_some() {
        return Err(AuthApiError::UsernameTaken);
    }

    let user = users.create_user(input).await?;

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

    transaction.commit().await?;

    Ok(user)
}

/// SeaORM 的 SQLite 事务默认延迟拿写锁；这里先执行无害写入，让首个用户判断串行化。
async fn acquire_registration_write_lock(transaction: &impl ConnectionTrait) -> Result<(), DbErr> {
    transaction
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "UPDATE auth_settings SET value = value WHERE key = 'access_token_ttl_seconds'"
                .to_owned(),
        ))
        .await?;

    Ok(())
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
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
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
