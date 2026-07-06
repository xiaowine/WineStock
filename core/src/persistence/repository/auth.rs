use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    DbErr, EntityTrait, QueryFilter, QueryOrder, Set, Statement,
};

use crate::persistence::entity::{auth_setting, auth_signing_key};

/// 鉴权启动使用的 repository，负责数据库托管设置和签名密钥读取。
pub(crate) struct AuthRepository<'db> {
    database: &'db DatabaseConnection,
}

impl<'db> AuthRepository<'db> {
    /// 创建绑定到同一个 SeaORM 连接的鉴权仓储。
    pub(crate) fn new(database: &'db DatabaseConnection) -> Self {
        Self { database }
    }

    /// 插入缺省鉴权设置，数据库已有值必须保留。
    pub(crate) async fn insert_default_settings(
        &self,
        defaults: &[(&str, &str)],
    ) -> Result<(), DbErr> {
        for (key, value) in defaults {
            let setting = auth_setting::ActiveModel {
                key: Set((*key).to_owned()),
                value: Set((*value).to_owned()),
                ..Default::default()
            };

            match auth_setting::Entity::insert(setting)
                .on_conflict(
                    OnConflict::column(auth_setting::Column::Key)
                        .do_nothing()
                        .to_owned(),
                )
                .exec(self.database)
                .await
            {
                // SeaORM 在 SQLite DO NOTHING 未插入时返回 RecordNotInserted，这里代表已有值应保留。
                Ok(_) | Err(DbErr::RecordNotInserted) => {}
                Err(source) => return Err(source),
            }
        }

        Ok(())
    }

    /// 读取单个鉴权设置的字符串值。
    pub(crate) async fn setting_value(&self, key: &str) -> Result<Option<String>, DbErr> {
        Ok(auth_setting::Entity::find_by_id(key.to_owned())
            .one(self.database)
            .await?
            .map(|setting| setting.value))
    }

    /// 读取当前 active 签名密钥。
    pub(crate) async fn active_signing_key(
        &self,
    ) -> Result<Option<auth_signing_key::Model>, DbErr> {
        auth_signing_key::Entity::find()
            .filter(auth_signing_key::Column::Status.eq("active"))
            .order_by_desc(auth_signing_key::Column::ActivatedAt)
            .order_by_desc(auth_signing_key::Column::Id)
            .one(self.database)
            .await
    }

    /// 创建新的 active 签名密钥，调用前应确认数据库中没有 active 密钥。
    pub(crate) async fn create_active_signing_key(
        &self,
        key_id: String,
        algorithm: &str,
        key_material: String,
    ) -> Result<auth_signing_key::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let active_key = auth_signing_key::ActiveModel {
            key_id: Set(key_id),
            algorithm: Set(algorithm.to_owned()),
            key_material: Set(key_material),
            status: Set("active".to_owned()),
            created_at: Set(now.clone()),
            activated_at: Set(Some(now)),
            retired_at: Set(None),
            ..Default::default()
        };

        let result = auth_signing_key::Entity::insert(active_key)
            .exec(self.database)
            .await?;

        auth_signing_key::Entity::find_by_id(result.last_insert_id)
            .one(self.database)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created auth signing key".to_owned()))
    }

    /// 判断是否已经存在任何用户，用于首次管理员初始化判断。
    pub(crate) async fn has_any_user(&self) -> Result<bool, DbErr> {
        // v1 schema 总是包含 users 表，首次管理员判断只需要检查是否已有用户。
        let row = self
            .database
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT EXISTS(SELECT 1 FROM users LIMIT 1) AS has_user".to_owned(),
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("users existence query".to_owned()))?;
        let has_user: i64 = row.try_get("", "has_user")?;

        Ok(has_user != 0)
    }
}

/// 从 SQLite 读取统一时间戳，避免 Rust 进程时间和数据库默认时间格式不一致。
pub(crate) async fn sqlite_now(database: &impl ConnectionTrait) -> Result<String, DbErr> {
    let row = database
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now') AS current_time".to_owned(),
        ))
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("SQLite current timestamp".to_owned()))?;

    row.try_get("", "current_time")
}
