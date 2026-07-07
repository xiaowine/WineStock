//! 仓储层对处理函数暴露业务语义，避免业务代码直接散写 SeaORM 查询。
//!
//! 当前仓储命名直接对齐 `auth/users/rbac` 模块，避免继续保留 `identity` 中间层目录。

mod auth_repo;
#[allow(dead_code)]
mod file_object;
mod rbac_repo;
mod refresh_token_repo;
mod time;
mod user_repo;
mod validation;

#[allow(unused_imports)]
pub(crate) use auth_repo::AuthRepository;
#[allow(unused_imports)]
pub(crate) use rbac_repo::{RbacRepository, RolePermissionSyncMode};
#[allow(unused_imports)]
pub(crate) use refresh_token_repo::{CreateRefreshToken, RefreshTokenRepository};
pub(crate) use time::{sqlite_now, sqlite_time_after_seconds};
#[allow(unused_imports)]
pub(crate) use user_repo::{CreateUser, UserRepository};

#[cfg(test)]
#[path = "../../tests/persistence_repository.rs"]
mod tests;
