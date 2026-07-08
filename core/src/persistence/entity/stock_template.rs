//! `stock_templates` 表的 SeaORM Entity。
//!
//! 该表保存库存模板基础资料。字段定义、物品引用检查和软删除规则由 stock repository
//! 在业务语义层处理，Entity 只负责表字段映射。

use sea_orm::entity::prelude::*;

/// 库存模板基础资料表。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于物品分类和模板字段引用。
    pub id: i64,

    /// 模板名称；未软删除记录由数据库局部唯一索引保证唯一。
    pub name: String,

    /// 模板说明，可为空。
    pub description: Option<String>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,

    /// 软删除时间；为空表示当前有效。
    pub deleted_at: Option<String>,
}

/// 模板与字段关系由仓储层按业务场景查询，Entity 不直接承载跨表流程。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
