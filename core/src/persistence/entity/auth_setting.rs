//! `auth_settings` 表的 SeaORM Entity。
//!
//! 该表保存数据库托管的鉴权配置。JSON 启动配置不承载 token TTL、
//! refresh token 轮换或签名密钥等安全相关运行时设置。

use sea_orm::entity::prelude::*;

/// 鉴权策略键值表，保存 token TTL、refresh token 轮换等数据库托管设置。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    /// 设置键，例如访问令牌过期秒数或刷新令牌轮换开关。
    pub key: String,

    /// 设置值，按业务读取时再解析成具体类型。
    pub value: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 当前表没有声明 SeaORM 级联关系，跨表约束由迁移和仓储层表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
