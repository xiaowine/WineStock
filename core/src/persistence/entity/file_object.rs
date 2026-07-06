//! `storage_file_objects` 表的 SeaORM Entity。
//!
//! 该表只映射文件元数据。文件二进制内容属于平台提供的 `files/` 目录，不进入 SQLite。

use sea_orm::entity::prelude::*;

/// 文件对象元数据表，不保存文件二进制内容。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "storage_file_objects")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于后续 API 引用文件元数据。
    pub id: i64,

    /// 文件内容的 SHA-256 摘要，用于去重、校验和按内容查询。
    pub sha256: String,

    /// 文件 MIME 类型；调用方无法判断时允许为空。
    pub mime_type: Option<String>,

    /// 文件大小，单位字节，不允许为负数。
    pub size_bytes: i64,

    /// 文件在 `files/` 目录下的相对存储路径。
    pub storage_path: String,

    /// 上传或导入时的原始文件名，仅作为展示信息。
    pub original_name: Option<String>,

    /// 元数据创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 文件所有者用户 ID；用户删除后允许置空保留文件记录。
    pub owner_user_id: Option<i64>,
}

/// 文件所有者关系先由数据库外键约束表达，仓储层对外只暴露元数据查询。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
