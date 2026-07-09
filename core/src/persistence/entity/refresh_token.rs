//! `auth_refresh_tokens` 表的 SeaORM Entity。
//!
//! 该表只保存刷新令牌哈希和设备元数据。明文令牌、签发流程和轮换事务属于鉴权服务或 repository。

use sea_orm::entity::prelude::*;

/// 刷新令牌表，保存哈希、设备信息、过期时间和吊销状态。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_refresh_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 数据库自增主键，用于内部定位令牌记录。
    pub id: i64,

    /// 令牌所属用户 ID，删除用户时级联删除令牌。
    pub user_id: i64,

    /// 刷新令牌哈希值，明文令牌不得入库。
    pub token_hash: String,

    /// 设备名称，便于用户识别登录来源。
    pub device_name: String,

    /// 客户端类型，例如 desktop、android 或 web。
    pub client_kind: String,

    /// App 版本号，用于定位登录来源的客户端版本。
    pub app_version: String,

    /// Refresh token 格式版本，由服务端当前 token 生成规则决定。
    pub refresh_token_version: String,

    /// 令牌创建时间，使用 SQLite UTC 字符串格式。
    pub created_at: String,

    /// 令牌过期时间，使用 SQLite UTC 字符串格式。
    pub expires_at: String,

    /// 最近一次使用时间，未使用过时为空。
    pub last_used_at: Option<String>,

    /// 吊销时间；为空表示当前未被吊销。
    pub revoked_at: Option<String>,

    /// 轮换后替代该令牌的新令牌 ID；普通登出吊销时为空。
    pub replaced_by_token_id: Option<i64>,
}

/// 刷新令牌与用户的关系先由外键约束表达，轮换事务由仓储层控制。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
