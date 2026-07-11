//! `stock_item_categories` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，只描述物品归类信息；它不拥有属性模板或入库字段定义。

use sea_orm::entity::prelude::*;

/// 物品分类表记录。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_item_categories")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 分类名称，未软删除记录内唯一。
    pub name: String,
    /// 分类说明。
    pub description: Option<String>,
    /// 分类展示顺序。
    pub sort_order: i32,
    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
    /// 软删除时间；为空表示当前有效。
    pub deleted_at: Option<String>,
}

/// 分类引用由仓储查询和数据库外键表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
