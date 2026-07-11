//! `storage_inbound_file_bindings` 表的 SeaORM Entity。
//!
//! 本实体属于 core 持久化层，记录单张图片被哪个入库明细属性占用。
//! 它不保存文件内容，也不把客户端本地路径写入数据库。

use sea_orm::entity::prelude::*;

/// 入库明细属性与文件对象的一对一绑定记录。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "storage_inbound_file_bindings")]
pub struct Model {
    /// 数据库自增主键。
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 已上传文件对象 ID；同一文件只能绑定一次。
    #[sea_orm(unique)]
    pub file_object_id: i64,

    /// 文件所属的入库明细属性 ID；同一属性只允许绑定一张图片。
    #[sea_orm(unique)]
    pub inbound_order_item_attribute_id: i64,

    /// 绑定创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,
}

/// 绑定关系由数据库外键约束表达，业务查询集中在 repository。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
