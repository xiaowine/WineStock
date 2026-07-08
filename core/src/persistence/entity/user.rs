//! `auth_users` 表的 SeaORM Entity。
//!
//! 该表是账号体系基础表。权限分配和登录流程不在 Entity 中实现，由 migration 外键和 repository 查询表达。

use sea_orm::entity::prelude::*;

/// 用户表，当前仅承载鉴权启动和后续账号体系所需的基础字段。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于内部关联权限、令牌和文件所有者。
    pub id: i64,

    /// 登录用户名，数据库中保持唯一。
    pub username: String,

    /// 密码哈希，不保存明文密码。
    pub password_hash: String,

    /// 展示名称；为空时可回退使用用户名。
    pub display_name: Option<String>,

    /// 用户状态，当前允许 `active` 或 `disabled`。
    pub status: String,

    /// 用户创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 用户最近更新时间，使用 SQLite UTC 字符串格式。
    pub updated_at: String,
}

/// 用户与权限的多对多关系通过 `auth_user_permission_assignments` 表和仓储层查询表达。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
