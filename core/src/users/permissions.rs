//! users 模块权限常量。
//!
//! 本模块属于 `users` 业务层，负责定义用户域需要声明给安全层和 RBAC 启动逻辑的稳定权限代码。
//! 它不负责角色模板或权限分配存储。

/// 允许创建新用户的权限代码。
pub(crate) const REGISTER_USER_PERMISSION: &str = "user.register";

/// 用户、角色和权限管理权限代码。
pub(crate) const MANAGE_USER_PERMISSION: &str = "user.manage";
