use sea_orm::entity::prelude::*;

/// JWT 访问令牌签名密钥表，每次启动复用当前启用密钥。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_signing_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，只用于内部关联和排序。
    pub id: i64,

    /// 对外暴露的密钥标识，写入 JWT header 的 kid。
    pub key_id: String,

    /// 签名算法标识，当前默认使用 HS256。
    pub algorithm: String,

    /// 签名密钥材料，日志和 Debug 输出不得直接暴露。
    pub key_material: String,

    /// 密钥状态，当前允许 `active` 或 `retired`。
    pub status: String,

    /// 密钥创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 密钥启用时间；active 密钥必须有该值。
    pub activated_at: Option<String>,

    /// 密钥退役时间；retired 密钥必须有该值。
    pub retired_at: Option<String>,
}

/// 当前表没有声明 SeaORM 级联关系，启用密钥唯一性由 SQLite 局部唯一索引保证。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
