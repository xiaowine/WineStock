//! `stock_item_attributes` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，保存物品固有属性；它不保存本次入库批次状态。

use sea_orm::entity::prelude::*;

/// 单个物品属性记录。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_item_attributes")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 所属物品 ID。
    pub item_id: i64,
    /// 可选模板字段来源；自定义属性为空。
    pub template_field_id: Option<i64>,
    /// 属性名称，同一物品内唯一。
    pub field_name: String,
    /// 稳定属性类型代码。
    pub field_type: String,
    /// JSON 编码后的属性值。
    pub value_json: String,
    /// 可选计量单位。
    pub unit: Option<String>,
    /// 属性展示顺序。
    pub sort_order: i32,
    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 属性所属物品和模板来源由数据库外键表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
