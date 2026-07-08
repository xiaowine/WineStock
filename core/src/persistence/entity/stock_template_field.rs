//! `stock_template_fields` 表的 SeaORM Entity。
//!
//! 该表保存库存模板字段定义。字段类型组合、select 选项和默认值规则由 stock 服务层校验，
//! Entity 只负责持久化映射。

use sea_orm::entity::prelude::*;

/// 库存模板字段定义表。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_template_fields")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于模板字段排序和内部引用。
    pub id: i64,

    /// 所属模板 ID。
    pub template_id: i64,

    /// 字段名称，同一模板内唯一。
    pub field_name: String,

    /// 字段类型，只允许 migration 中声明的稳定代码。
    pub field_type: String,

    /// 是否必填，SQLite 中使用 0/1 保存。
    pub required: i32,

    /// 是否可用于搜索，SQLite 中使用 0/1 保存。
    pub searchable: i32,

    /// select 等字段类型使用的候选值 JSON。
    pub options_json: Option<String>,

    /// 默认值，可为空。
    pub default_value: Option<String>,

    /// 字段显示和校验排序。
    pub sort_order: i32,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 字段所属模板由数据库外键约束表达，仓储层负责组合模板详情。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
