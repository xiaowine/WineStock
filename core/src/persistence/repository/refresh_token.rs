use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};

use crate::persistence::entity::refresh_token;

use super::auth::sqlite_now;

/// 创建刷新令牌时写入数据库的安全元数据，明文令牌不进入 SQLite。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateRefreshToken {
    /// 令牌所属用户 ID，必须指向已存在用户。
    pub user_id: i64,

    /// 刷新令牌哈希值，明文令牌不得入库。
    pub token_hash: String,

    /// 设备名称，便于用户识别登录来源。
    pub device_name: Option<String>,

    /// 客户端类型，用于区分桌面、Android 或其他调用方。
    pub client_kind: Option<String>,

    /// 令牌过期时间，使用 SQLite UTC 字符串格式。
    pub expires_at: String,
}

/// 刷新令牌仓储层管理查询、吊销和轮换的事务边界。
pub(crate) struct RefreshTokenRepository<'db> {
    database: &'db DatabaseConnection,
}

impl<'db> RefreshTokenRepository<'db> {
    /// 创建绑定到同一个 SeaORM 连接的刷新令牌仓储。
    pub(crate) fn new(database: &'db DatabaseConnection) -> Self {
        Self { database }
    }

    /// 创建刷新令牌记录，只保存令牌哈希和设备元数据。
    pub(crate) async fn create(
        &self,
        input: CreateRefreshToken,
    ) -> Result<refresh_token::Model, DbErr> {
        create_on_connection(self.database, input).await
    }

    /// 按哈希查找未吊销的刷新令牌。
    pub(crate) async fn find_active_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<refresh_token::Model>, DbErr> {
        find_active_by_hash_on_connection(self.database, token_hash).await
    }

    /// 吊销当前 active 刷新令牌；不存在或已吊销时返回 false。
    pub(crate) async fn revoke(&self, token_hash: &str) -> Result<bool, DbErr> {
        let txn = self.database.begin().await?;
        let revoked = revoke_on_transaction(&txn, token_hash).await?;
        txn.commit().await?;

        Ok(revoked)
    }

    /// 在同一事务中吊销旧令牌并创建新令牌。
    pub(crate) async fn rotate(
        &self,
        old_token_hash: &str,
        new_token: CreateRefreshToken,
    ) -> Result<Option<refresh_token::Model>, DbErr> {
        // 先吊销旧令牌再创建新令牌，必须放在同一个事务里避免双活令牌。
        let txn = self.database.begin().await?;
        let existing = find_active_by_hash_on_connection(&txn, old_token_hash).await?;
        let rotated = if existing.is_some() {
            revoke_on_transaction(&txn, old_token_hash).await?;
            Some(create_on_connection(&txn, new_token).await?)
        } else {
            None
        };
        txn.commit().await?;

        Ok(rotated)
    }
}

async fn create_on_connection<C>(
    connection: &C,
    input: CreateRefreshToken,
) -> Result<refresh_token::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = sqlite_now(connection).await?;
    let active_token = refresh_token::ActiveModel {
        user_id: Set(input.user_id),
        token_hash: Set(input.token_hash),
        device_name: Set(input.device_name),
        client_kind: Set(input.client_kind),
        created_at: Set(now),
        expires_at: Set(input.expires_at),
        last_used_at: Set(None),
        revoked_at: Set(None),
        ..Default::default()
    };
    let result = refresh_token::Entity::insert(active_token)
        .exec(connection)
        .await?;

    refresh_token::Entity::find_by_id(result.last_insert_id)
        .one(connection)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("created refresh token".to_owned()))
}

async fn find_active_by_hash_on_connection<C>(
    connection: &C,
    token_hash: &str,
) -> Result<Option<refresh_token::Model>, DbErr>
where
    C: ConnectionTrait,
{
    refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(token_hash))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .one(connection)
        .await
}

async fn revoke_on_transaction(
    transaction: &DatabaseTransaction,
    token_hash: &str,
) -> Result<bool, DbErr> {
    // 已吊销或不存在的令牌视为没有状态变更，调用方可据此返回幂等结果。
    let Some(token) = find_active_by_hash_on_connection(transaction, token_hash).await? else {
        return Ok(false);
    };
    let now = sqlite_now(transaction).await?;
    let mut active: refresh_token::ActiveModel = token.into();
    active.revoked_at = Set(Some(now));

    refresh_token::Entity::update(active)
        .exec(transaction)
        .await?;

    Ok(true)
}
