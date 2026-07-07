//! rbac 授权模型模块入口。
//!
//! 本模块属于 `core axum library` 的授权模型层，负责内置角色/权限定义和启动补齐逻辑。
//! 它不签发 JWT，也不承担 HTTP 请求处理。

mod bootstrap;
mod policy;

pub use bootstrap::RbacBootstrapError;

pub(crate) use bootstrap::bootstrap_builtin_rbac;
pub(crate) use policy::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME};
