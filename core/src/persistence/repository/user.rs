//! 用户 repository。
//!
//! 本模块属于 core 持久化层，封装用户创建、用户查询和 RBAC 权限读取。
//! HTTP handler 和鉴权流程不应直接拼接用户、角色、权限关联表查询。

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};

use crate::persistence::entity::user;

use super::auth::sqlite_now;

/// 创建用户的最小输入，密码哈希由上层鉴权流程生成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateUser {
    /// 登录用户名，数据库中保持唯一。
    pub username: String,

    /// 已完成算法处理的密码哈希，不接受明文密码。
    pub password_hash: String,

    /// 展示名称；为空时可回退使用用户名。
    pub display_name: Option<String>,
}

/// 用户仓储层封装账号查询和 RBAC 权限读取。
pub(crate) struct UserRepository<'db> {
    database: &'db DatabaseConnection,
}

impl<'db> UserRepository<'db> {
    /// 创建绑定到同一个 SeaORM 连接的用户仓储。
    pub(crate) fn new(database: &'db DatabaseConnection) -> Self {
        Self { database }
    }

    /// 创建 active 用户，并使用数据库统一时间戳填充创建和更新时间。
    pub(crate) async fn create_user(&self, input: CreateUser) -> Result<user::Model, DbErr> {
        let now = sqlite_now(self.database).await?;
        let active_user = user::ActiveModel {
            username: Set(input.username),
            password_hash: Set(input.password_hash),
            display_name: Set(input.display_name),
            status: Set("active".to_owned()),
            created_at: Set(now.clone()),
            updated_at: Set(now),
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
        user::Entity::find_by_id(id).one(self.database).await
    }

    /// 按唯一用户名查询用户。
    pub(crate) async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<user::Model>, DbErr> {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(self.database)
            .await
    }

    /// 查询用户经由角色获得的权限代码列表。
    pub(crate) async fn list_user_permissions(&self, user_id: i64) -> Result<Vec<String>, DbErr> {
        // 权限列表跨三张关联表，保留为仓储层内部 SQL，避免处理函数依赖表结构。
        let rows = self
            .database
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Sqlite,
                r#"
                SELECT DISTINCT auth_permissions.code AS code
                FROM auth_permissions
                INNER JOIN auth_role_permission_assignments
                    ON auth_role_permission_assignments.permission_id = auth_permissions.id
                INNER JOIN auth_user_role_assignments
                    ON auth_user_role_assignments.role_id = auth_role_permission_assignments.role_id
                WHERE auth_user_role_assignments.user_id = ?
                ORDER BY auth_permissions.code
                "#,
                [user_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| row.try_get("", "code"))
            .collect()
    }
}
