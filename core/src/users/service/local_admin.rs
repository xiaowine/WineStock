//! 本机免登录标记用户的开通、自愈与占位密码标记。
//!
//! 本模块属于 `users` 业务服务层，服务 self-hosted 静默会话：空库首次换取时自动开通
//! 用户指定用户名，标记用户被停用/软删除/收权后自愈恢复，并维护"密码仍为随机占位"标记。
//! 它不校验换取凭据，也不签发 token；那些属于 `auth` 业务层。

use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use serde_json::json;

use crate::{
    persistence::{
        entity::user,
        repository::{
            AuditRepository, AuthRepository, CreateUser, RbacRepository, RecordAuditEvent,
            UserRepository,
        },
    },
    rbac::builtin_permission_codes,
    security::{create_password_hash, random_urlsafe, AuthApiError},
};

use super::{register::acquire_registration_write_lock, validation::normalize_username};

/// 数据库托管鉴权设置：本机静默会话签发目标的用户 ID。
pub(crate) const LOCAL_AUTO_LOGIN_USER_ID_SETTING: &str = "local_auto_login_user_id";

/// 数据库托管鉴权设置：标记用户密码是否仍为自动开通时的随机占位值。
pub(crate) const LOCAL_AUTO_LOGIN_PASSWORD_PLACEHOLDER_SETTING: &str =
    "local_auto_login_password_placeholder";

/// 解析本机静默会话的目标用户；空库时自动开通，标记用户异常时自愈。
///
/// 与首用户注册共用同一把写锁串行化，避免"浏览器注册首用户"与"壳内首次换取"并发时
/// 产生两个初始全权限用户。已有用户但没有标记（存量库未转换）时拒绝，不做启发式误绑。
pub(crate) async fn resolve_local_auto_login_user(
    database: &DatabaseConnection,
    initial_username: Option<&str>,
) -> Result<user::Model, AuthApiError> {
    let transaction = database.begin().await?;
    acquire_registration_write_lock(&transaction).await?;

    let auth = AuthRepository::new(&transaction);
    let user = match read_marker(&auth).await? {
        Some(marked_user_id) => heal_marked_user(&transaction, marked_user_id).await?,
        None => {
            if auth.has_any_user().await? {
                return Err(AuthApiError::LocalSessionUnavailable);
            }
            let username = initial_username.ok_or(AuthApiError::LocalInitialUserRequired)?;
            let username = normalize_username(username)?;
            provision_local_user(&transaction, &username).await?
        }
    };

    transaction.commit().await?;
    Ok(user)
}

/// 标记用户改密后清除占位标记；非标记用户改密不产生任何写入。
pub(crate) async fn clear_password_placeholder_if_marked(
    connection: &impl ConnectionTrait,
    user_id: i64,
) -> Result<(), AuthApiError> {
    let auth = AuthRepository::new(connection);
    if read_marker(&auth).await? == Some(user_id) {
        auth.set_setting_value(LOCAL_AUTO_LOGIN_PASSWORD_PLACEHOLDER_SETTING, "false")
            .await?;
    }

    Ok(())
}

/// 当前用户是否处于"标记用户且密码仍为占位"状态；该状态允许免旧密码设置新密码。
pub(crate) async fn password_placeholder_active(
    connection: &impl ConnectionTrait,
    user_id: i64,
) -> Result<bool, AuthApiError> {
    let auth = AuthRepository::new(connection);
    if read_marker(&auth).await? != Some(user_id) {
        return Ok(false);
    }

    Ok(auth
        .get_setting_value(LOCAL_AUTO_LOGIN_PASSWORD_PLACEHOLDER_SETTING)
        .await?
        .as_deref()
        == Some("true"))
}

/// 读取标记用户 ID；设置值损坏按未标记处理，走"存量库未转换"的拒绝路径。
async fn read_marker(
    auth: &AuthRepository<'_, impl ConnectionTrait>,
) -> Result<Option<i64>, AuthApiError> {
    Ok(auth
        .get_setting_value(LOCAL_AUTO_LOGIN_USER_ID_SETTING)
        .await?
        .and_then(|value| value.parse::<i64>().ok()))
}

/// 空库自动开通用户：随机占位密码 + 全部内置权限 + 标记与占位设置 + 审计。
async fn provision_local_user(
    transaction: &(impl ConnectionTrait + Send + Sync),
    username: &str,
) -> Result<user::Model, AuthApiError> {
    let users = UserRepository::new(transaction);
    let rbac = RbacRepository::new(transaction);
    let auth = AuthRepository::new(transaction);
    let audit = AuditRepository::new(transaction);

    // 占位密码只用于满足非空约束，明文当场丢弃，无人可用它登录。
    let placeholder_password = random_urlsafe(32).map_err(AuthApiError::Random)?;
    let password_hash = create_password_hash(&placeholder_password)?;
    let user = users
        .create_user(CreateUser {
            username: username.to_owned(),
            password_hash,
        })
        .await?;

    grant_all_builtin_permissions(&rbac, user.id).await?;
    auth.set_setting_value(LOCAL_AUTO_LOGIN_USER_ID_SETTING, &user.id.to_string())
        .await?;
    auth.set_setting_value(LOCAL_AUTO_LOGIN_PASSWORD_PLACEHOLDER_SETTING, "true")
        .await?;
    audit
        .record(RecordAuditEvent {
            user_id: Some(user.id),
            entity_type: "user".to_owned(),
            entity_id: Some(user.id),
            action: "created".to_owned(),
            details: Some(json!({
                "username": user.username.clone(),
                "first_user": true,
                "mode": "local_auto_provision"
            })),
        })
        .await?;

    Ok(user)
}

/// 自愈标记用户：恢复停用/软删除状态并补齐全部内置权限，任何修复都写审计留痕。
///
/// 依据"本机物理持有者即最高权限"：SQLite 文件本就在设备持有者手中，不自愈只会让
/// 本机界面陷入无法逃生的错误态（本地静默模式没有登录页）。
async fn heal_marked_user(
    transaction: &(impl ConnectionTrait + Send + Sync),
    marked_user_id: i64,
) -> Result<user::Model, AuthApiError> {
    let users = UserRepository::new(transaction);
    let rbac = RbacRepository::new(transaction);
    let audit = AuditRepository::new(transaction);

    let user = users
        .find_by_id_any(marked_user_id)
        .await?
        .ok_or(AuthApiError::LocalSessionUnavailable)?;
    let needs_restore = user.status != "active" || user.deleted_at.is_some();
    let user = if needs_restore {
        users.restore(user).await?
    } else {
        user
    };

    let builtin_codes = builtin_permission_codes();
    let current_permissions = rbac.list_user_permissions(user.id).await?;
    let needs_permissions = builtin_codes
        .iter()
        .any(|code| !current_permissions.iter().any(|owned| owned == code));
    if needs_permissions {
        grant_all_builtin_permissions(&rbac, user.id).await?;
    }

    if needs_restore || needs_permissions {
        audit
            .record(RecordAuditEvent {
                user_id: Some(user.id),
                entity_type: "user".to_owned(),
                entity_id: Some(user.id),
                action: "updated".to_owned(),
                details: Some(json!({
                    "mode": "local_auto_login_heal",
                    "restored": needs_restore,
                    "permissions_restored": needs_permissions
                })),
            })
            .await?;
    }

    Ok(user)
}

async fn grant_all_builtin_permissions(
    rbac: &RbacRepository<'_, impl ConnectionTrait>,
    user_id: i64,
) -> Result<(), AuthApiError> {
    let permission_codes = builtin_permission_codes();
    let permission_ids = rbac
        .find_permission_ids_by_codes(&permission_codes)
        .await?
        .ok_or(AuthApiError::PermissionNotFound)?;
    rbac.replace_user_permissions(user_id, &permission_ids)
        .await?;

    Ok(())
}
