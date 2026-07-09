//! 用户注册服务。
//!
//! 本模块属于 `users` 业务服务层，负责注册用例、首个用户判断、首个用户权限分配和注册审计写入。
//! 它不处理登录会话、用户管理列表或 HTTP 路由。

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};
use serde_json::json;

use crate::{
    auth::{AuthRegisterRequest, AuthUserResponse},
    persistence::{
        entity::user,
        repository::{
            AuditRepository, AuthRepository, CreateUser, RbacRepository, RecordAuditEvent,
            UserRepository,
        },
    },
    rbac::builtin_permission_codes,
    security::{create_password_hash, AuthApiError, CurrentUser},
    state::CoreState,
    users::REGISTER_USER_PERMISSION,
};

use super::{response::load_user_response, validation::normalize_username};

/// 执行注册用例；当数据库尚无用户时，首个用户会获得全部内置权限，并记录创建审计事件。
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

/// 在同一事务中完成注册、首个用户判断、首个用户权限分配和审计写入，避免并发首登产生多个初始权限用户。
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
    let audit = AuditRepository::new(&transaction);
    let has_users = auth_repository.has_any_user().await?;

    if has_users {
        let Some(current_user) = current_user else {
            return Err(AuthApiError::InvalidAccessToken);
        };
        if !current_user.has_permission(REGISTER_USER_PERMISSION) {
            return Err(AuthApiError::PermissionDenied);
        }
    }

    if users.find_by_username(&input.username).await?.is_some() {
        return Err(AuthApiError::UsernameTaken);
    }

    let user = users.create_user(input).await?;

    if !has_users {
        let permission_codes = builtin_permission_codes();
        let permission_ids = rbac
            .find_permission_ids_by_codes(&permission_codes)
            .await?
            .ok_or(AuthApiError::PermissionNotFound)?;
        rbac.replace_user_permissions(user.id, &permission_ids)
            .await?;
    }

    let operator_user_id = current_user.map(|user| user.user_id).unwrap_or(user.id);
    audit
        .record(RecordAuditEvent {
            user_id: Some(operator_user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(user.id),
            action: "created".to_owned(),
            details: Some(json!({
                "username": user.username.clone(),
                "first_user": !has_users
            })),
        })
        .await?;

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
