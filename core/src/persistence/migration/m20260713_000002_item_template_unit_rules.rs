//! 为物品属性模板字段增加显式单位规则。
//!
//! 本 migration 属于 core 持久化层，只扩展物品属性模板字段；入库模板不保存逐字段单位规则。

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

/// 非破坏性增加单位模式、固定单位和单位候选项。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for statement in ADD_UNIT_RULE_COLUMNS {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for statement in DROP_UNIT_RULE_COLUMNS {
            manager
                .get_connection()
                .execute_unprepared(statement)
                .await?;
        }
        Ok(())
    }
}

const ADD_UNIT_RULE_COLUMNS: &[&str] = &[
    "ALTER TABLE stock_item_attribute_template_fields ADD COLUMN unit_mode TEXT NOT NULL DEFAULT 'none' CHECK (unit_mode IN ('none', 'fixed', 'select', 'custom'))",
    "ALTER TABLE stock_item_attribute_template_fields ADD COLUMN fixed_unit TEXT",
    "ALTER TABLE stock_item_attribute_template_fields ADD COLUMN unit_options_json TEXT",
];

const DROP_UNIT_RULE_COLUMNS: &[&str] = &[
    "ALTER TABLE stock_item_attribute_template_fields DROP COLUMN unit_options_json",
    "ALTER TABLE stock_item_attribute_template_fields DROP COLUMN fixed_unit",
    "ALTER TABLE stock_item_attribute_template_fields DROP COLUMN unit_mode",
];
