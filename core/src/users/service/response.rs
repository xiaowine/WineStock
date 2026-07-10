//! 用户服务响应组装。
//!
//! 本模块属于 `users` 业务服务层，负责把用户与权限仓储记录转换为 API 响应。
//! 它不处理注册、改密或权限替换事务。

use sea_orm::ConnectionTrait;

use crate::{
    auth::AuthUserResponse,
    persistence::{
        entity::user,
        repository::{PermissionRecord, RbacRepository},
    },
    security::AuthApiError,
    users::controller,
};

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
pub(super) async fn load_admin_user_response(
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
    user: &user::Model,
) -> Result<controller::UserAdminResponse, AuthApiError> {
    Ok(controller::UserAdminResponse {
        id: user.id,
        username: user.username.clone(),
        status: controller::UserStatus::from_code(&user.status)?,
        permissions: rbac.list_user_permissions(user.id).await?,
        password_change_required: user.password_change_required,
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    })
}

/// 把权限定义记录转换为管理接口响应。
pub(super) fn permission_response(record: PermissionRecord) -> controller::PermissionResponse {
    controller::PermissionResponse {
        code: record.code,
        description: record.description,
    }
}
