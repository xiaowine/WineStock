//! 用户管理服务。
//!
//! 本模块属于 `users` 业务服务层，负责用户列表、详情、状态、软删除、权限替换、临时密码重置和权限定义查询。
//! 它不处理注册首个用户或当前用户自助改密。

use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::json;

use crate::{
    persistence::{
        entity::user,
        repository::{
            AuditRepository, ListUsers, RbacRepository, RecordAuditEvent, RefreshTokenRepository,
            UserRepository,
        },
    },
    security::{create_password_hash, AuthApiError, CurrentUser},
    state::CoreState,
    users::{
        controller, READ_USER_PERMISSION_DEFINITION_PERMISSION, UPDATE_USER_PERMISSIONS_PERMISSION,
    },
};

use super::{
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{load_admin_user_response, permission_response},
    validation::{normalize_optional_status, normalize_optional_text, normalize_permission_codes},
};

/// 分页查询用户管理列表。
pub(crate) async fn list_users(
    state: &CoreState,
    query: controller::UserListQuery,
) -> Result<PaginatedResponse<controller::UserAdminResponse>, AuthApiError> {
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
) -> Result<controller::UserAdminResponse, AuthApiError> {
    let users = UserRepository::new(state.database());
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    let rbac = RbacRepository::new(state.database());
    load_admin_user_response(&rbac, &user).await
}

/// 更新用户状态；禁止停用当前操作者，并保护最后一个 active 权限管理员。
pub(crate) async fn update_user_status(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::UserStatusUpdateRequest,
) -> Result<controller::UserAdminResponse, AuthApiError> {
    if id == current_user.user_id && request.status == controller::UserStatus::Disabled {
        return Err(AuthApiError::SelfStatusUpdateForbidden);
    }

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

/// 软删除其他用户；同时停用账号、吊销 refresh token，并保护最后一个 active 权限管理员。
pub(crate) async fn delete_user(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<(), AuthApiError> {
    if id == current_user.user_id {
        return Err(AuthApiError::SelfDeleteForbidden);
    }

    let transaction = state.database().begin().await?;
    let users = UserRepository::new(&transaction);
    let rbac = RbacRepository::new(&transaction);
    let refresh_tokens = RefreshTokenRepository::new(&transaction);
    let audit = AuditRepository::new(&transaction);
    let user = users
        .find_by_id(id)
        .await?
        .ok_or(AuthApiError::UserNotFound)?;
    ensure_user_can_lose_permission_management(&rbac, &user, Some("disabled"), None).await?;
    let previous_status = user.status.clone();
    let deleted = users.soft_delete(user).await?;
    refresh_tokens.revoke_active_for_user(deleted.id).await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(deleted.id),
            action: "deleted".to_owned(),
            details: Some(json!({
                "mode": "soft_delete",
                "previous_status": previous_status
            })),
        })
        .await?;
    transaction.commit().await?;

    Ok(())
}

/// 整体替换用户权限；保护当前操作者的关键权限，并禁止移除最后一个 active 权限管理员的管理权限。
pub(crate) async fn update_user_permissions(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::UserPermissionsUpdateRequest,
) -> Result<controller::UserAdminResponse, AuthApiError> {
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
    let previous_permissions = rbac.list_user_permissions(user.id).await?;
    ensure_self_protected_permissions_unchanged(
        current_user,
        user.id,
        &previous_permissions,
        &permission_codes,
    )?;
    ensure_user_can_lose_permission_management(&rbac, &user, None, Some(&permission_codes)).await?;
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

/// 当前操作者更新自己的权限时，两项关键权限在更新前后必须保持不变。
fn ensure_self_protected_permissions_unchanged(
    current_user: &CurrentUser,
    target_user_id: i64,
    current_permissions: &[String],
    next_permissions: &[String],
) -> Result<(), AuthApiError> {
    if current_user.user_id != target_user_id {
        return Ok(());
    }

    for protected_permission in [
        UPDATE_USER_PERMISSIONS_PERMISSION,
        READ_USER_PERMISSION_DEFINITION_PERMISSION,
    ] {
        let currently_has_permission = current_permissions
            .iter()
            .any(|permission| permission == protected_permission);
        let will_have_permission = next_permissions
            .iter()
            .any(|permission| permission == protected_permission);
        if currently_has_permission != will_have_permission {
            return Err(AuthApiError::SelfProtectedPermissionsUpdateForbidden);
        }
    }

    Ok(())
}

/// 为其他用户设置临时密码；禁止作用于当前操作者，目标用户下次登录后必须改密。
pub(crate) async fn reset_user_password(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
    request: controller::UserPasswordResetRequest,
) -> Result<(), AuthApiError> {
    if request.password.is_empty() {
        return Err(AuthApiError::InvalidRequest);
    }
    if id == current_user.user_id {
        return Err(AuthApiError::SelfPasswordResetForbidden);
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
) -> Result<Vec<controller::PermissionResponse>, AuthApiError> {
    Ok(RbacRepository::new(state.database())
        .list_permissions()
        .await?
        .into_iter()
        .map(permission_response)
        .collect())
}

/// 禁止移除最后一个 active 用户的权限管理能力。
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
        .any(|permission| permission == UPDATE_USER_PERMISSIONS_PERMISSION)
    {
        return Ok(());
    }

    let will_be_active = next_status.unwrap_or(&user.status) == "active";
    let will_manage_permissions = next_permissions
        .map(|permissions| {
            permissions
                .iter()
                .any(|permission| permission == UPDATE_USER_PERMISSIONS_PERMISSION)
        })
        .unwrap_or(true);
    if (!will_be_active || !will_manage_permissions)
        && !rbac
            .has_other_active_user_with_permission(user.id, UPDATE_USER_PERMISSIONS_PERMISSION)
            .await?
    {
        return Err(AuthApiError::LastPermissionManagerRequired);
    }

    Ok(())
}
