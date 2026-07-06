mod m20260706_000001_initial_schema;

use sea_orm_migration::prelude::*;

/// core 内置迁移集合，平台壳只通过 `StorageConfig.auto_migrate` 决定是否执行。
pub(super) struct Migrator;

#[sea_orm_migration::async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260706_000001_initial_schema::Migration)]
    }
}
