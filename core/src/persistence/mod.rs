//! core 本地持久化入口，集中管理 SQLite 连接、迁移、实体和 repository。
//!
//! 本模块属于 `core` 的基础设施层，负责持久化总入口和领域仓储归组。
//! 连接与 migration 仍是全局基础设施，entity/repository 则按当前业务模块提供直白命名。

mod connection;
pub(crate) mod entity;
mod migration;
pub(crate) mod repository;

pub(crate) use connection::{migrate_storage_schema, open_sqlite_storage};
pub use connection::{StorageBootstrapError, StorageRuntime};
