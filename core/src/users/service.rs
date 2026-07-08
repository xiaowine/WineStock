//! users 模块业务服务。
//!
//! 本模块属于 `users` 业务层，负责注册、当前用户查询、自助改密码、用户管理以及用户响应组装。
//! 它不负责路由级鉴权条件，也不直接解析 bearer token。

use winestock_shared::{AuthRegisterRequest, AuthUserResponse};

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

use crate::{
    persistence::{
        entity::user,
        repository::{
            AuditRepository, AuthRepository, CreateUser, ListUsers, PermissionRecord,
            RbacRepository, RecordAuditEvent, RefreshTokenRepository, UserRepository,
        },
    },
    rbac::builtin_permission_codes,
    security::{create_password_hash, verify_password, AuthApiError, CurrentUser},
    state::CoreState,
};

/// 用户管理分页默认页码。
pub(crate) const DEFAULT_PAGE: u64 = 1;

/// 用户管理分页默认每页数量。
pub(crate) const DEFAULT_PAGE_SIZE: u64 = 50;

/// 用户管理分页最大每页数量。
pub(crate) const MAX_PAGE_SIZE: u64 = 200;

/// 用户管理分页响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PaginatedResponse<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 满足查询条件的总记录数。
    pub total: u64,

    /// 当前页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 总页数；无数据时返回 0。
    pub total_pages: u64,
}

/// 执行注册用例；当数据库尚无用户时，首个用户会获得全部内置权限。
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

/// 在同一事务中完成注册、首个用户判断和首个用户权限分配，避免并发首登产生多个初始权限用户。
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
        let permission_codes = builtin_permission_codes();
        let permission_ids = rbac
            .find_permission_ids_by_codes(&permission_codes)
            .await?
            .ok_or(AuthApiError::PermissionNotFound)?;
        rbac.replace_user_permissions(user.id, &permission_ids)
            .await?;
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

/// 根据当前认证上下文读取数据库中的最新用户和权限快照。
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

/// 分页查询用户管理列表。
pub(crate) async fn list_users(
    state: &CoreState,
    query: super::controller::UserListQuery,
) -> Result<PaginatedResponse<super::controller::UserAdminResponse>, AuthApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(MAX_PAGE_SIZE);
    let repository = UserRepository::new(state.database());
    let result = repository
        .list_users(ListUsers {
            page,
            page_size,
            search: normalize_optional_text(query.search)?,
            status: normalize_optional_status(query.status)?,
        })
        .await?;
    let rbac = RbacRepository::new(state.database());
    let mut items = Vec::with_capacity(result.items.len());
    for user in result.items {
        items.push(load_admin_user_response(&rbac, &user).await?);
    }

    Ok(PaginatedResponse {
        items,
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询单个用户管理详情。
pub(crate) async fn get_user(
    state: &CoreState,
    id: i64,
) -> Result<super::controller::UserAdminResponse, AuthApiError> {
    let users = UserRepository::new(state.database());
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    let rbac = RbacRepository::new(state.database());
    load_admin_user_response(&rbac, &user).await
}

/// 更新用户状态；禁止禁用最后一个拥有用户权限管理能力的 active 用户。
pub(crate) async fn update_user_status(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: super::controller::UserStatusUpdateRequest,
) -> Result<super::controller::UserAdminResponse, AuthApiError> {
    let status = request.status.as_code().to_owned();
    let transaction = state.database().begin().await?;
    let users = UserRepository::new(&transaction);
    let rbac = RbacRepository::new(&transaction);
    let audit = AuditRepository::new(&transaction);
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    ensure_user_can_lose_permission_management(&rbac, &user, Some(&status), None).await?;
    let previous_status = user.status.clone();
    let updated = users.update_status(user, status.clone()).await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(updated.id),
            action: "updated".to_owned(),
            details: Some(json!({
                "field": "status",
                "previous_status": previous_status,
                "new_status": status
            })),
        })
        .await?;
    let response = load_admin_user_response(&rbac, &updated).await?;
    transaction.commit().await?;

    Ok(response)
}

/// 整体替换用户权限；禁止移除最后一个 active 权限管理员的管理权限。
pub(crate) async fn update_user_permissions(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: super::controller::UserPermissionsUpdateRequest,
) -> Result<super::controller::UserAdminResponse, AuthApiError> {
    let permission_codes = normalize_permission_codes(request.permissions)?;
    let transaction = state.database().begin().await?;
    let users = UserRepository::new(&transaction);
    let rbac = RbacRepository::new(&transaction);
    let audit = AuditRepository::new(&transaction);
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    let permission_ids = rbac
        .find_permission_ids_by_codes(&permission_codes)
        .await?
        .ok_or(AuthApiError::PermissionNotFound)?;
    ensure_user_can_lose_permission_management(&rbac, &user, None, Some(&permission_codes)).await?;
    let previous_permissions = rbac.list_user_permissions(user.id).await?;
    rbac.replace_user_permissions(user.id, &permission_ids)
        .await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(user.id),
            action: "updated".to_owned(),
            details: Some(json!({
                "field": "permissions",
                "previous_permissions": previous_permissions,
                "new_permissions": permission_codes
            })),
        })
        .await?;
    let response = load_admin_user_response(&rbac, &user).await?;
    transaction.commit().await?;

    Ok(response)
}

/// 当前用户修改自己的密码；必须先校验当前密码，审计详情不得包含明文密码或哈希。
pub(crate) async fn change_own_password(
    state: &CoreState,
    current_user: &CurrentUser,
    request: super::controller::UserPasswordChangeRequest,
) -> Result<(), AuthApiError> {
    if request.current_password.is_empty() || request.new_password.is_empty() {
        return Err(AuthApiError::InvalidRequest);
    }
    let transaction = state.database().begin().await?;
    let users = UserRepository::new(&transaction);
    let audit = AuditRepository::new(&transaction);
    let user = users
        .find_by_id(current_user.user_id)
        .await?
        .ok_or(AuthApiError::InvalidAccessToken)?;
    if user.status != "active" {
        return Err(AuthApiError::InvalidAccessToken);
    }
    if !verify_password(&request.current_password, &user.password_hash) {
        return Err(AuthApiError::InvalidCredentials);
    }

    let password_hash = create_password_hash(&request.new_password)?;
    let updated = users
        .update_password_hash(user, password_hash, false)
        .await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(updated.id),
            action: "updated".to_owned(),
            details: Some(json!({
                "field": "password",
                "mode": "self_change"
            })),
        })
        .await?;
    transaction.commit().await?;

    Ok(())
}

/// 拥有重置密码权限的用户设置目标用户临时密码；目标用户下次登录后必须改密。
pub(crate) async fn reset_user_password(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: super::controller::UserPasswordResetRequest,
) -> Result<(), AuthApiError> {
    if request.password.is_empty() {
        return Err(AuthApiError::InvalidRequest);
    }
    let password_hash = create_password_hash(&request.password)?;
    let transaction = state.database().begin().await?;
    let users = UserRepository::new(&transaction);
    let refresh_tokens = RefreshTokenRepository::new(&transaction);
    let audit = AuditRepository::new(&transaction);
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    let updated = users
        .update_password_hash(user, password_hash, true)
        .await?;
    refresh_tokens.revoke_active_for_user(updated.id).await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(updated.id),
            action: "updated".to_owned(),
            details: Some(json!({
                "field": "password",
                "mode": "admin_temporary_password",
                "password_change_required": true
            })),
        })
        .await?;
    transaction.commit().await?;

    Ok(())
}

/// 查询权限列表。
pub(crate) async fn list_permissions(
    state: &CoreState,
) -> Result<Vec<super::controller::PermissionResponse>, AuthApiError> {
    Ok(RbacRepository::new(state.database())
        .list_permissions()
        .await?
        .into_iter()
        .map(permission_response)
        .collect())
}

/// 组装 API 返回和 JWT claims 共享的用户权限快照。
pub(crate) async fn load_user_response(
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
    user: &user::Model,
) -> Result<AuthUserResponse, AuthApiError> {
    Ok(AuthUserResponse {
        id: user.id.to_string(),
        username: user.username.clone(),
        permissions: rbac.list_user_permissions(user.id).await?,
        password_change_required: user.password_change_required,
    })
}

/// 组装用户管理接口响应，包含账号状态和审计所需时间字段。
async fn load_admin_user_response(
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
    user: &user::Model,
) -> Result<super::controller::UserAdminResponse, AuthApiError> {
    Ok(super::controller::UserAdminResponse {
        id: user.id,
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        status: super::controller::UserStatus::from_code(&user.status)?,
        permissions: rbac.list_user_permissions(user.id).await?,
        password_change_required: user.password_change_required,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    })
}

fn permission_response(record: PermissionRecord) -> super::controller::PermissionResponse {
    super::controller::PermissionResponse {
        code: record.code,
        description: record.description,
    }
}

async fn ensure_user_can_lose_permission_management(
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
    user: &user::Model,
    next_status: Option<&str>,
    next_permissions: Option<&[String]>,
) -> Result<(), AuthApiError> {
    if user.status != "active" {
        return Ok(());
    }
    let current_permissions = rbac.list_user_permissions(user.id).await?;
    if !current_permissions
        .iter()
        .any(|permission| permission == super::UPDATE_USER_PERMISSIONS_PERMISSION)
    {
        return Ok(());
    }

    let will_be_active = next_status.unwrap_or(&user.status) == "active";
    let will_manage_permissions = next_permissions
        .map(|permissions| {
            permissions
                .iter()
                .any(|permission| permission == super::UPDATE_USER_PERMISSIONS_PERMISSION)
        })
        .unwrap_or(true);
    if (!will_be_active || !will_manage_permissions)
        && !rbac
            .has_other_active_user_with_permission(
                user.id,
                super::UPDATE_USER_PERMISSIONS_PERMISSION,
            )
            .await?
    {
        return Err(AuthApiError::LastPermissionManagerRequired);
    }

    Ok(())
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

fn normalize_optional_text(value: Option<String>) -> Result<Option<String>, AuthApiError> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(AuthApiError::InvalidRequest)
            } else {
                Ok(trimmed.to_owned())
            }
        })
        .transpose()
}

fn normalize_optional_status(value: Option<String>) -> Result<Option<String>, AuthApiError> {
    value
        .map(|value| normalize_status_code(&value).map(ToOwned::to_owned))
        .transpose()
}

fn normalize_permission_codes(values: Vec<String>) -> Result<Vec<String>, AuthApiError> {
    let mut codes = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AuthApiError::InvalidRequest);
        }
        codes.insert(trimmed.to_owned());
    }

    Ok(codes.into_iter().collect())
}

fn normalize_status_code(value: &str) -> Result<&'static str, AuthApiError> {
    match value.trim() {
        "active" => Ok("active"),
        "disabled" => Ok("disabled"),
        _ => Err(AuthApiError::InvalidRequest),
    }
}

fn total_pages(total: u64, page_size: u64) -> u64 {
    if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    }
}
