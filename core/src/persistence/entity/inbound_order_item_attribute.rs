//! `stock_inbound_order_item_attributes` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，保存某条入库明细的本次收货属性，不保存物品固有资料。

use sea_orm::entity::prelude::*;

/// 入库明细属性记录。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_inbound_order_item_attributes")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 所属入库明细 ID。
    pub inbound_order_item_id: i64,
    /// 可选入库模板字段来源。
    pub template_field_id: Option<i64>,
    /// 属性名称，同一入库明细内唯一。
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
}

/// 属性所属入库明细和模板来源由数据库外键表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
