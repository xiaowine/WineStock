//! 仓储层对处理函数暴露业务语义，避免业务代码直接散写 SeaORM 查询。

mod auth;

#[allow(dead_code)]
mod file_object;

#[allow(dead_code)]
mod refresh_token;

#[allow(dead_code)]
mod rbac;

mod time;

#[allow(dead_code)]
mod user;

pub(crate) use auth::AuthRepository;
pub(crate) use rbac::{RbacRepository, RolePermissionSyncMode};
pub(crate) use refresh_token::{CreateRefreshToken, RefreshTokenRepository};
pub(crate) use time::{sqlite_now, sqlite_time_after_seconds};
pub(crate) use user::CreateUser;
pub(crate) use user::UserRepository;

#[cfg(test)]
#[path = "../../tests/persistence_repository.rs"]
mod tests;
