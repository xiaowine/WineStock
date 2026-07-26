//! `stock_item_attribute_templates` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，描述可选物品属性预设；它不能限制物品添加自定义属性。

use sea_orm::entity::prelude::*;

/// 物品属性模板基础记录。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_item_attribute_templates")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 模板名称，未软删除记录内唯一。
    pub name: String,
    /// 模板说明。
    pub description: Option<String>,
    /// 是否为全站默认模板；服务层保证有效模板中至多一个为真。
    pub is_default: bool,
    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
    /// 软删除时间；为空表示当前有效。
    pub deleted_at: Option<String>,
}

/// 模板字段由仓储层组合读取。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
