//! `stock_item_attribute_definitions` 表的 SeaORM Entity。

use sea_orm::entity::prelude::*;

/// 模板属性与物品私有自定义属性共用的定义。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_item_attribute_definitions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub template_id: Option<i64>,
    pub owner_item_id: Option<i64>,
    pub field_name: String,
    pub field_type: String,
    pub required: i32,
    pub searchable: i32,
    pub options_json: Option<String>,
    pub default_value: Option<String>,
    pub unit_mode: String,
    pub fixed_unit: Option<String>,
    pub unit_options_json: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
