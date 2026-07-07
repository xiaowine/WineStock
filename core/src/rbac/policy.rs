//! rbac 模块策略常量。
//!
//! 本模块属于 `rbac` 授权模型层，负责保存稳定的角色代码，以及当前尚未拥有正式业务模块时的
//! 共用权限代码。`stock` 领域正式落地后，应优先把库存权限移动到对应业务模块的 `permissions.rs`。

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

/// 查看库存数据权限代码。
pub(crate) const STOCK_READ_PERMISSION: &str = "stock.read";

/// 创建或修改库存数据权限代码。
pub(crate) const STOCK_WRITE_PERMISSION: &str = "stock.write";
