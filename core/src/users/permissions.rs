//! users 模块权限常量。
//!
//! 本模块属于 `users` 业务层，负责定义用户域需要声明给安全层和 RBAC 启动逻辑的稳定权限代码。
//! 它不负责权限定义补齐或权限分配存储。

/// 允许创建新用户的权限代码。
pub(crate) const REGISTER_USER_PERMISSION: &str = "user.register";

/// 查看用户列表和用户详情的权限代码。
pub(crate) const READ_USER_PERMISSION: &str = "user.read";

/// 启用或停用用户账号的权限代码。
pub(crate) const UPDATE_USER_STATUS_PERMISSION: &str = "user.status.update";

/// 整体替换用户权限的权限代码。
pub(crate) const UPDATE_USER_PERMISSIONS_PERMISSION: &str = "user.permissions.update";

/// 查看权限定义的权限代码。
pub(crate) const READ_USER_PERMISSION_DEFINITION_PERMISSION: &str = "user.permission.read";

/// 管理员直接重置其他用户密码的权限代码。
pub(crate) const RESET_USER_PASSWORD_PERMISSION: &str = "user.password.reset";
