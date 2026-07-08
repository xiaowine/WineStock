//! rbac 模块策略常量。
//!
//! 本模块属于 `rbac` 授权模型层，负责保存稳定的角色代码。
//! 业务权限代码应放在各自业务模块的 `permissions.rs`，避免 RBAC 层拥有业务语义。

/// 内置管理员角色代码，用于首次用户分配。
pub(crate) const ADMIN_ROLE_CODE: &str = "admin";

/// 内置管理员角色默认名称。
pub(crate) const ADMIN_ROLE_NAME: &str = "Admin";

/// 内置员工角色代码。
pub(crate) const STAFF_ROLE_CODE: &str = "staff";

/// 内置员工角色默认名称
pub(crate) const STAFF_ROLE_NAME: &str = "Staff";

/// 内置只读角色代码。
pub(crate) const VIEWER_ROLE_CODE: &str = "viewer";

/// 内置只读角色默认名称。
pub(crate) const VIEWER_ROLE_NAME: &str = "Viewer";
