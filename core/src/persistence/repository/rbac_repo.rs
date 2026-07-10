//! rbac 模块 repository。
//!
//! 本模块属于 `core` 的持久化层，封装权限定义和用户权限分配。
//! 用户账号仓储不拥有权限关联表结构，鉴权和业务处理函数也不应直接拼接这些关联查询。

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};

/// 权限定义读取模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionRecord {
    /// 权限数据库 ID。
    pub id: i64,

    /// 稳定权限代码。
    pub code: String,

    /// 权限说明。
    pub description: Option<String>,
}

/// 权限仓储层封装权限定义、分配与查询。
pub(crate) struct RbacRepository<'db, C = DatabaseConnection>
where
    C: ConnectionTrait,
{
    database: &'db C,
}

impl<'db, C> RbacRepository<'db, C>
where
    C: ConnectionTrait,
{
    /// 创建绑定到同一个 SeaORM 连接的权限仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

    /// 查询用户直接拥有的权限代码列表。
    pub(crate) async fn list_user_permissions(&self, user_id: i64) -> Result<Vec<String>, DbErr> {
        // 用户权限只通过直接分配表读取，避免业务层重新引入角色继承语义。
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT DISTINCT auth_permissions.code AS code
                FROM auth_permissions
                INNER JOIN auth_user_permission_assignments
                    ON auth_user_permission_assignments.permission_id = auth_permissions.id
                WHERE auth_user_permission_assignments.user_id = ?
                ORDER BY auth_permissions.code
                "#,
                [user_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| row.try_get("", "code"))
            .collect()
    }

    /// 查询全部权限定义。
    pub(crate) async fn list_permissions(&self) -> Result<Vec<PermissionRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                SELECT id, code, description
                FROM auth_permissions
                ORDER BY code
                "#
                .to_owned(),
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(PermissionRecord {
                    id: row.try_get("", "id")?,
                    code: row.try_get("", "code")?,
                    description: row.try_get("", "description")?,
                })
            })
            .collect()
    }

    /// 按权限代码批量解析权限 ID；任一代码不存在时返回空。
    pub(crate) async fn find_permission_ids_by_codes(
        &self,
        codes: &[String],
    ) -> Result<Option<Vec<i64>>, DbErr> {
        let mut permission_ids = Vec::with_capacity(codes.len());
        for code in codes {
            let Some(permission_id) = self.find_permission_id_by_code(code).await? else {
                return Ok(None);
            };
            permission_ids.push(permission_id);
        }

        Ok(Some(permission_ids))
    }

    /// 确保指定权限存在，并返回权限 ID。
    pub(crate) async fn ensure_permission(
        &self,
        code: &str,
        description: Option<&str>,
    ) -> Result<i64, DbErr> {
        // 权限定义属于内置基础数据，已存在时不覆盖管理员后续调整。
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO auth_permissions (code, description)
                VALUES (?, ?)
                ON CONFLICT(code) DO NOTHING
                "#,
                [code.into(), description.into()],
            ))
            .await?;

        self.find_permission_id_by_code(code)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("auth permission".to_owned()))
    }

    /// 给用户直接分配权限；已有分配保持不变。
    pub(crate) async fn assign_permission_to_user(
        &self,
        user_id: i64,
        permission_id: i64,
    ) -> Result<(), DbErr> {
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO auth_user_permission_assignments (user_id, permission_id)
                VALUES (?, ?)
                ON CONFLICT(user_id, permission_id) DO NOTHING
                "#,
                [user_id.into(), permission_id.into()],
            ))
            .await?;

        Ok(())
    }

    /// 整体替换用户权限；调用方应在事务内先完成防锁死业务校验。
    pub(crate) async fn replace_user_permissions(
        &self,
        user_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), DbErr> {
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM auth_user_permission_assignments WHERE user_id = ?",
                [user_id.into()],
            ))
            .await?;

        for permission_id in permission_ids {
            self.assign_permission_to_user(user_id, *permission_id)
                .await?;
        }

        Ok(())
    }

    /// 判断除指定用户外是否还有 active 用户拥有目标权限，用于避免管理操作锁死系统。
    pub(crate) async fn has_other_active_user_with_permission(
        &self,
        excluded_user_id: i64,
        permission_code: &str,
    ) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(DISTINCT auth_users.id) AS count
                FROM auth_users
                INNER JOIN auth_user_permission_assignments
                    ON auth_user_permission_assignments.user_id = auth_users.id
                INNER JOIN auth_permissions
                    ON auth_permissions.id = auth_user_permission_assignments.permission_id
                WHERE auth_users.status = 'active'
                    AND auth_users.deleted_at IS NULL
                    AND auth_users.id <> ?
                    AND auth_permissions.code = ?
                "#,
                [excluded_user_id.into(), permission_code.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("active permission holder count".to_owned()))?;

        let count: i64 = row.try_get("", "count")?;
        Ok(count > 0)
    }

    /// 按权限代码查询权限 ID。
    async fn find_permission_id_by_code(&self, code: &str) -> Result<Option<i64>, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT id FROM auth_permissions WHERE code = ?",
                [code.into()],
            ))
            .await?;

        row.map(|row| row.try_get("", "id")).transpose()
    }
}
