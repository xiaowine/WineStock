//! auth 模块鉴权 repository。
//!
//! 本模块属于 `core` 的持久化层，封装鉴权设置、签名密钥和首次管理员判断所需查询。
//! 调用方不需要知道 `auth_settings`、`auth_signing_keys` 和 `auth_users` 的具体查询细节。

use sea_orm::{
    sea_query::OnConflict, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::persistence::entity::{auth_setting, auth_signing_key, user};

use crate::persistence::repository::time::sqlite_now;

/// 鉴权启动使用的 repository，负责数据库托管设置和签名密钥读取。
pub(crate) struct AuthRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> AuthRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的鉴权仓储。
    pub(crate) fn new(database: &'db C) -> Self {
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

    /// 写入或覆盖单个鉴权设置的字符串值。
    pub(crate) async fn set_setting_value(&self, key: &str, value: &str) -> Result<(), DbErr> {
        let setting = auth_setting::ActiveModel {
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
            ..Default::default()
        };

        match auth_setting::Entity::insert(setting)
            .on_conflict(
                OnConflict::column(auth_setting::Column::Key)
                    .update_column(auth_setting::Column::Value)
                    .to_owned(),
            )
            .exec(self.database)
            .await
        {
            Ok(_) | Err(DbErr::RecordNotInserted) => Ok(()),
            Err(source) => Err(source),
        }
    }

    /// 读取单个鉴权设置的字符串值。
    pub(crate) async fn get_setting_value(&self, key: &str) -> Result<Option<String>, DbErr> {
        Ok(auth_setting::Entity::find_by_id(key.to_owned())
            .one(self.database)
            .await?
            .map(|setting| setting.value))
    }

    /// 读取当前 active 签名密钥。
    pub(crate) async fn find_active_signing_key(
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
        // v1 schema 总是包含 auth_users 表，首次管理员判断只需要检查是否已有用户。
        user::Entity::find().exists(self.database).await
    }
}
