//! users 模块用户 repository。
//!
//! 本模块属于 `core` 的持久化层，封装用户创建、查询、状态更新、密码哈希和强制改密标记更新。
//! 权限定义和分配关系属于 RBAC repository，不应混入账号仓储。

use crate::validation::validate_not_blank;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, Set, Statement, Value,
};

use crate::persistence::entity::user;

use crate::persistence::repository::{time::sqlite_now, validation::validate_repository_input};

/// 创建用户的最小输入，密码哈希由上层鉴权流程生成。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateUser {
    /// 登录用户名，数据库中保持唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub username: String,

    /// 已完成算法处理的密码哈希，不接受明文密码。
    #[garde(length(min = 1, max = 512), custom(validate_not_blank))]
    pub password_hash: String,
}

/// 用户分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListUsers {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 用户名模糊搜索关键字。
    pub search: Option<String>,

    /// 按用户状态筛选。
    pub status: Option<String>,
}

/// 用户分页查询结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserPage {
    /// 当前页用户记录。
    pub items: Vec<user::Model>,

    /// 满足条件的用户总数。
    pub total: u64,
}

/// 用户仓储层封装账号创建和查询。
pub(crate) struct UserRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> UserRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的用户仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

    /// 创建 active 用户，并使用数据库统一时间戳填充创建和更新时间。
    pub(crate) async fn create_user(&self, input: CreateUser) -> Result<user::Model, DbErr> {
        validate_repository_input(&input)?;
        let now = sqlite_now(self.database).await?;
        let active_user = user::ActiveModel {
            username: Set(input.username),
            password_hash: Set(input.password_hash),
            status: Set("active".to_owned()),
            password_change_required: Set(false),
            created_at: Set(now.clone()),
            updated_at: Set(now),
            deleted_at: Set(None),
            ..Default::default()
        };
        let result = user::Entity::insert(active_user)
            .exec(self.database)
            .await?;

        self.find_by_id(result.last_insert_id)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("created user".to_owned()))
    }

    /// 按数据库主键查询用户。
    pub(crate) async fn find_by_id(&self, id: i64) -> Result<Option<user::Model>, DbErr> {
        user::Entity::find_by_id(id)
            .filter(user::Column::DeletedAt.is_null())
            .one(self.database)
            .await
    }

    /// 按数据库主键查询用户，包含已软删除记录；本机免登录自愈需要感知被删标记用户。
    pub(crate) async fn find_by_id_any(&self, id: i64) -> Result<Option<user::Model>, DbErr> {
        user::Entity::find_by_id(id).one(self.database).await
    }

    /// 恢复被停用或软删除的用户为 active；本机免登录自愈专用，调用方负责写审计。
    pub(crate) async fn restore(&self, user: user::Model) -> Result<user::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let mut active: user::ActiveModel = user.into();
        active.status = Set("active".to_owned());
        active.deleted_at = Set(None);
        active.updated_at = Set(now);
        active.update(self.database).await
    }

    /// 按唯一用户名查询用户。
    pub(crate) async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<user::Model>, DbErr> {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .filter(user::Column::DeletedAt.is_null())
            .one(self.database)
            .await
    }

    /// 判断用户名是否已被任何账号占用；软删除账号仍保留登录标识，避免审计身份混淆。
    pub(crate) async fn username_exists(&self, username: &str) -> Result<bool, DbErr> {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .exists(self.database)
            .await
    }

    /// 分页查询用户，只返回账号基础记录。
    pub(crate) async fn list_users(&self, input: ListUsers) -> Result<UserPage, DbErr> {
        let limit = input.page_size as i64;
        let offset = ((input.page.saturating_sub(1)) * input.page_size) as i64;
        let (join_clause, where_clause, values) = list_users_query_parts(&input);
        let total = self
            .count_users(&join_clause, &where_clause, values.clone())
            .await?;
        let mut data_values = values;
        data_values.push(limit.into());
        data_values.push(offset.into());
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    r#"
                    SELECT DISTINCT
                        auth_users.id,
                        auth_users.username,
                        auth_users.password_hash,
                        auth_users.status,
                        auth_users.password_change_required,
                        auth_users.created_at,
                        auth_users.updated_at,
                        auth_users.deleted_at
                    FROM auth_users
                    {join_clause}
                    {where_clause}
                    ORDER BY auth_users.id ASC
                    LIMIT ? OFFSET ?
                    "#
                ),
                data_values,
            ))
            .await?;

        let items = rows
            .into_iter()
            .map(|row| {
                Ok(user::Model {
                    id: row.try_get("", "id")?,
                    username: row.try_get("", "username")?,
                    password_hash: row.try_get("", "password_hash")?,
                    status: row.try_get("", "status")?,
                    password_change_required: row.try_get("", "password_change_required")?,
                    created_at: row.try_get("", "created_at")?,
                    updated_at: row.try_get("", "updated_at")?,
                    deleted_at: row.try_get("", "deleted_at")?,
                })
            })
            .collect::<Result<Vec<_>, DbErr>>()?;

        Ok(UserPage { items, total })
    }

    /// 更新用户状态，并返回更新后的用户记录。
    pub(crate) async fn update_status(
        &self,
        user: user::Model,
        status: String,
    ) -> Result<user::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let mut active: user::ActiveModel = user.into();
        active.status = Set(status);
        active.updated_at = Set(now);
        active.update(self.database).await
    }

    /// 更新用户密码哈希和强制改密标记，并返回更新后的用户记录。
    pub(crate) async fn update_password_hash(
        &self,
        user: user::Model,
        password_hash: String,
        password_change_required: bool,
    ) -> Result<user::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let mut active: user::ActiveModel = user.into();
        active.password_hash = Set(password_hash);
        active.password_change_required = Set(password_change_required);
        active.updated_at = Set(now);
        active.update(self.database).await
    }

    /// 将用户标记为已删除并停用账号；调用方负责在同一事务内吊销会话和写审计。
    pub(crate) async fn soft_delete(&self, user: user::Model) -> Result<user::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let mut active: user::ActiveModel = user.into();
        active.status = Set("disabled".to_owned());
        active.updated_at = Set(now.clone());
        active.deleted_at = Set(Some(now));
        active.update(self.database).await
    }

    async fn count_users(
        &self,
        join_clause: &str,
        where_clause: &str,
        values: Vec<Value>,
    ) -> Result<u64, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    r#"
                    SELECT COUNT(DISTINCT auth_users.id) AS count
                    FROM auth_users
                    {join_clause}
                    {where_clause}
                    "#
                ),
                values,
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("user count".to_owned()))?;

        row.try_get::<i64>("", "count").map(|count| count as u64)
    }
}

fn list_users_query_parts(input: &ListUsers) -> (String, String, Vec<Value>) {
    let joins = String::new();
    let mut clauses = vec!["auth_users.deleted_at IS NULL"];
    let mut values = Vec::new();

    if let Some(search) = input.search.as_ref() {
        clauses.push("auth_users.username LIKE ?");
        let pattern = format!("%{search}%");
        values.push(pattern.into());
    }

    if let Some(status) = input.status.as_ref() {
        clauses.push("auth_users.status = ?");
        values.push(status.clone().into());
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };

    (joins, where_clause, values)
}
