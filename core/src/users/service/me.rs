//! 当前用户自助服务。
//!
//! 本模块属于 `users` 业务服务层，负责当前用户快照读取和自助改密码。
//! 它不处理用户管理、注册首个用户或权限定义列表。

use sea_orm::TransactionTrait;
use serde_json::json;

use crate::{
    auth::AuthUserResponse,
    persistence::repository::{AuditRepository, RbacRepository, RecordAuditEvent, UserRepository},
    security::{create_password_hash, verify_password, AuthApiError, CurrentUser},
    state::CoreState,
    users::controller,
};

use super::{
    local_admin::{clear_password_placeholder_if_marked, password_placeholder_active},
    response::load_user_response,
};

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

/// 当前用户修改自己的密码；必须先校验当前密码，审计详情不得包含明文密码或哈希。
///
/// 唯一例外：本机免登录标记用户的密码仍为随机占位值时（占位密码无人知晓），
/// 允许不提供当前密码直接设置新密码，成功后清除占位标记。
pub(crate) async fn change_own_password(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::UserPasswordChangeRequest,
) -> Result<(), AuthApiError> {
    if request.new_password.is_empty() {
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
    let placeholder_active = password_placeholder_active(&transaction, user.id).await?;
    if !placeholder_active {
        if request.current_password.is_empty() {
            return Err(AuthApiError::InvalidRequest);
        }
        if !verify_password(&request.current_password, &user.password_hash) {
            return Err(AuthApiError::InvalidCredentials);
        }
    }

    let password_hash = create_password_hash(&request.new_password)?;
    let updated = users
        .update_password_hash(user, password_hash, false)
        .await?;
    clear_password_placeholder_if_marked(&transaction, updated.id).await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(current_user.user_id),
            entity_type: "user".to_owned(),
            entity_id: Some(updated.id),
            action: "updated".to_owned(),
            details: Some(json!({
                "field": "password",
                "mode": if placeholder_active {
                    "local_placeholder_initial_set"
                } else {
                    "self_change"
                }
            })),
        })
        .await?;
    transaction.commit().await?;

    Ok(())
}
