//! users 模块业务服务入口。
//!
//! 本模块属于 `users` 业务层，负责汇总注册、当前用户、自助改密和用户管理用例。
//! 具体业务按子模块拆分；本模块不负责 bearer token 解析，也不直接暴露数据库表结构。

mod local_admin;
mod management;
mod me;
mod pagination;
mod register;
mod response;
mod validation;

pub(crate) use local_admin::{password_placeholder_active, resolve_local_auto_login_user};
pub(crate) use management::{
    delete_user, get_user, list_permissions, list_users, reset_user_password,
    update_user_permissions, update_user_status, update_user_username,
};
pub(crate) use me::{change_own_password, current_user};
pub(crate) use pagination::PaginatedResponse;
pub(crate) use register::register;
pub(crate) use response::load_user_response;
