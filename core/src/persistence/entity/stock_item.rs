//! `stock_items` 表的 SeaORM Entity。
//!
//! 该表保存库存物品基础资料。库存余额、批次扣减和审计写入不在 Entity 中实现，
//! 由 stock repository 和业务服务在事务边界内处理。

use sea_orm::entity::prelude::*;

/// 库存物品基础资料表。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于出入库明细、批次和替代料关系引用。
    pub id: i64,

    /// 物品名称，用于展示和列表搜索。
    pub name: String,

    /// 物品 SKU；未软删除记录由数据库局部唯一索引保证唯一。
    pub sku: String,

    /// 关联的物品分类 ID；分类删除后允许置空保留物品。
    pub category_id: Option<i64>,

    /// 可选物品属性模板 ID；模板只提供录入预设，不限制自定义属性。
    pub attribute_template_id: Option<i64>,

    /// 计量单位，例如个、米、KG 或件。
    pub unit: String,

    /// 物品描述，可为空。
    pub description: Option<String>,

    /// 参考单价，不参与已审批库存批次的成本回写。
    pub default_price: Option<f64>,

    /// 再订货点，低库存提醒由后续看板能力使用。
    pub reorder_point: Option<f64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,

    /// 软删除时间；为空表示当前有效。
    pub deleted_at: Option<String>,
}

/// 物品关联由仓储层按业务场景查询，Entity 不直接承载跨表流程。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
