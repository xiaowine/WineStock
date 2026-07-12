//! SeaORM migration 注册入口。
//!
//! 本模块属于 core 持久化层，只声明内置 migration 列表。
//! 是否执行 migration 由平台传入的 `StorageConfig.auto_migrate` 决定。

mod m20260706_000001_initial_schema;
mod m20260713_000002_item_template_unit_rules;

use sea_orm_migration::prelude::*;

/// core 内置迁移集合，平台壳只通过 `StorageConfig.auto_migrate` 决定是否执行。
pub(super) struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260706_000001_initial_schema::Migration),
            Box::new(m20260713_000002_item_template_unit_rules::Migration),
        ]
    }
}
