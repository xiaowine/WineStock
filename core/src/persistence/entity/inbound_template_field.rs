//! `stock_inbound_template_fields` 表的 SeaORM Entity。
//!
//! 本模块属于 core 持久化层，只映射入库模板字段；字段组合规则由 stock 服务校验。

use sea_orm::entity::prelude::*;

/// 入库模板字段定义。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_inbound_template_fields")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 所属入库模板 ID。
    pub template_id: i64,
    /// 字段名称，同一模板内唯一。
    pub field_name: String,
    /// 稳定字段类型代码。
    pub field_type: String,
    /// 是否必填，SQLite 使用 0/1 保存。
    pub required: i32,
    /// 是否允许参与搜索，SQLite 使用 0/1 保存。
    pub searchable: i32,
    /// `select` 候选值 JSON。
    pub options_json: Option<String>,
    /// 可选默认值。
    pub default_value: Option<String>,
    /// 字段展示顺序。
    pub sort_order: i32,
    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 字段所属关系由数据库外键表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
