//! `storage_item_file_bindings` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，记录物品文件属性与受控文件对象的绑定，不保存文件内容。

use sea_orm::entity::prelude::*;

/// 物品文件属性的一对一文件绑定。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "storage_item_file_bindings")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 文件对象 ID；同一文件只能被绑定一次。
    #[sea_orm(unique)]
    pub file_object_id: i64,
    /// 物品属性 ID；同一文件属性只允许一张图片。
    #[sea_orm(unique)]
    pub item_attribute_id: i64,
    /// 绑定创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
}

/// 绑定关系由数据库外键表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
