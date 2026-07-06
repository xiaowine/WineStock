//! core 本地持久化入口，集中管理 SQLite 连接、迁移、实体和 repository。

mod connection;
pub(crate) mod entity;
mod migration;
pub(crate) mod repository;

pub(crate) use connection::{migrate_storage_schema, open_sqlite_storage};
pub use connection::{StorageBootstrapError, StorageRuntime};
