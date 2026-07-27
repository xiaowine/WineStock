//! auth 模块刷新令牌 repository。
//!
//! 本模块属于 `core` 的持久化层，封装 refresh token 的创建、查询、吊销和轮换事务。
//! 明文令牌不进入本模块，调用方只能传入哈希和设备元数据。

use crate::validation::validate_not_blank;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};

use crate::persistence::entity::refresh_token;

use crate::persistence::repository::{time::sqlite_now, validation::validate_repository_input};

/// 创建刷新令牌时写入数据库的安全元数据，明文令牌不进入 SQLite。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateRefreshToken {
    /// 令牌所属用户 ID，必须指向已存在用户。
    #[garde(range(min = 1))]
    pub user_id: i64,

    /// 刷新令牌哈希值，明文令牌不得入库。
    #[garde(length(bytes, min = 1, max = 128), custom(validate_not_blank))]
    pub token_hash: String,

    /// 设备名称，便于用户识别登录来源。
    #[garde(length(utf16, min = 1, max = 64), custom(validate_not_blank))]
    pub device_name: String,

    /// 客户端类型，用于区分桌面、Android 或其他调用方。
    #[garde(length(bytes, min = 1, max = 32), custom(validate_not_blank))]
    pub client_kind: String,

    /// App 版本号，用于定位登录来源的客户端版本。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_not_blank))]
    pub app_version: String,

    /// Refresh token 格式版本，由服务端当前 token 生成规则决定。
    #[garde(length(bytes, min = 1, max = 32), custom(validate_not_blank))]
    pub refresh_token_version: String,

    /// 令牌过期时间，使用 SQLite UTC 字符串格式。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_not_blank))]
    pub expires_at: String,
}

/// 刷新令牌仓储层管理查询、吊销和轮换的事务边界。
pub(crate) struct RefreshTokenRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> RefreshTokenRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的刷新令牌仓储。
    pub(crate) fn new(database: &'db C) -> Self {
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
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn find_active_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<refresh_token::Model>, DbErr> {
        find_active_by_hash_on_connection(self.database, token_hash).await
    }

    /// 按哈希查找令牌记录，包含已吊销令牌，用于识别旧 refresh token 复用。
    pub(crate) async fn find_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<refresh_token::Model>, DbErr> {
        find_by_hash_on_connection(self.database, token_hash).await
    }

    /// 吊销指定用户的全部 active refresh token；管理员设置临时密码时用于强制重新登录。
    pub(crate) async fn revoke_active_for_user(&self, user_id: i64) -> Result<(), DbErr> {
        let now = sqlite_now(self.database).await?;
        self.database
            .execute(Statement::from_sql_and_values(
                self.database.get_database_backend(),
                r#"
                UPDATE auth_refresh_tokens
                SET revoked_at = ?
                WHERE user_id = ? AND revoked_at IS NULL
                "#,
                [now.into(), user_id.into()],
            ))
            .await?;

        Ok(())
    }
}

impl<'db> RefreshTokenRepository<'db, DatabaseConnection> {
    /// 吊销当前 active 刷新令牌；不存在或已吊销时返回 false。
    pub(crate) async fn revoke(&self, token_hash: &str) -> Result<bool, DbErr> {
        let txn = self.database.begin().await?;
        let revoked = revoke_on_transaction(&txn, token_hash, None).await?;
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
        let rotated = if let Some(old_token) = existing {
            let created = create_on_connection(&txn, new_token).await?;
            revoke_on_transaction(&txn, old_token_hash, Some(created.id)).await?;
            mark_last_used_on_transaction(&txn, old_token.id).await?;
            Some(created)
        } else {
            None
        };
        txn.commit().await?;

        Ok(rotated)
    }
}

/// 在指定连接或事务上创建刷新令牌记录，供普通创建和轮换事务复用。
async fn create_on_connection<C>(
    connection: &C,
    input: CreateRefreshToken,
) -> Result<refresh_token::Model, DbErr>
where
    C: ConnectionTrait,
{
    validate_repository_input(&input)?;
    let now = sqlite_now(connection).await?;
    let active_token = refresh_token::ActiveModel {
        user_id: Set(input.user_id),
        token_hash: Set(input.token_hash),
        device_name: Set(input.device_name),
        client_kind: Set(input.client_kind),
        app_version: Set(input.app_version),
        refresh_token_version: Set(input.refresh_token_version),
        created_at: Set(now),
        expires_at: Set(input.expires_at),
        last_used_at: Set(None),
        revoked_at: Set(None),
        replaced_by_token_id: Set(None),
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

/// 在指定连接或事务上查找未吊销令牌，轮换前必须用同一事务视图读取。
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

/// 在指定连接或事务上按哈希查找任意状态令牌。
async fn find_by_hash_on_connection<C>(
    connection: &C,
    token_hash: &str,
) -> Result<Option<refresh_token::Model>, DbErr>
where
    C: ConnectionTrait,
{
    refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(token_hash))
        .one(connection)
        .await
}

/// 在事务内标记刷新令牌为已吊销；调用方负责提交或回滚事务。
async fn revoke_on_transaction(
    transaction: &DatabaseTransaction,
    token_hash: &str,
    replaced_by_token_id: Option<i64>,
) -> Result<bool, DbErr> {
    // 已吊销或不存在的令牌视为没有状态变更，调用方可据此返回幂等结果。
    let Some(token) = find_active_by_hash_on_connection(transaction, token_hash).await? else {
        return Ok(false);
    };
    let now = sqlite_now(transaction).await?;
    let mut active: refresh_token::ActiveModel = token.into();
    active.revoked_at = Set(Some(now));
    active.replaced_by_token_id = Set(replaced_by_token_id);

    refresh_token::Entity::update(active)
        .exec(transaction)
        .await?;

    Ok(true)
}

/// 在事务内记录令牌最近使用时间；轮换时即使随后吊销，也能保留审计信息。
async fn mark_last_used_on_transaction(
    transaction: &DatabaseTransaction,
    token_id: i64,
) -> Result<(), DbErr> {
    let now = sqlite_now(transaction).await?;
    let Some(token) = refresh_token::Entity::find_by_id(token_id)
        .one(transaction)
        .await?
    else {
        return Ok(());
    };
    let mut active: refresh_token::ActiveModel = token.into();
    active.last_used_at = Set(Some(now));
    refresh_token::Entity::update(active)
        .exec(transaction)
        .await?;

    Ok(())
}
