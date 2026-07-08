//! rbac 模块 repository。
//!
//! 本模块属于 `core` 的持久化层，封装角色、权限、用户角色分配和角色权限分配。
//! 用户账号仓储不拥有 RBAC 表结构，鉴权和业务处理函数也不应直接拼接这些关联查询。

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};

/// 角色定义读取模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoleRecord {
    /// 角色数据库 ID。
    pub id: i64,

    /// 稳定角色代码。
    pub code: String,

    /// 角色名称。
    pub name: String,

    /// 角色说明。
    pub description: Option<String>,
}

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

/// 同步角色权限时对已有权限关系的处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RolePermissionSyncMode {
    /// 保留角色已有权限，只追加本次传入但尚未存在的权限。
    PreserveExisting,

    /// 先清空角色已有权限，再写入本次传入的权限集合。
    ReplaceExisting,
}

/// RBAC 仓储层封装角色和权限定义、分配与查询。
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
    /// 创建绑定到同一个 SeaORM 连接的 RBAC 仓储。
    pub(crate) fn new(database: &'db C) -> Self {
        Self { database }
    }

    /// 查询用户经由角色获得的权限代码列表。
    pub(crate) async fn list_user_permissions(&self, user_id: i64) -> Result<Vec<String>, DbErr> {
        // 权限列表跨三张关联表，保留为仓储层内部 SQL，避免处理函数依赖表结构。
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
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

    /// 查询用户直接分配到的角色代码列表。
    pub(crate) async fn list_user_roles(&self, user_id: i64) -> Result<Vec<String>, DbErr> {
        // 角色读取跨用户-角色关联表，保持在仓储层以隔离 RBAC 表结构。
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT DISTINCT auth_roles.code AS code
                FROM auth_roles
                INNER JOIN auth_user_role_assignments
                    ON auth_user_role_assignments.role_id = auth_roles.id
                WHERE auth_user_role_assignments.user_id = ?
                ORDER BY auth_roles.code
                "#,
                [user_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| row.try_get("", "code"))
            .collect()
    }

    /// 查询全部角色定义。
    pub(crate) async fn list_roles(&self) -> Result<Vec<RoleRecord>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                SELECT id, code, name, description
                FROM auth_roles
                ORDER BY code
                "#
                .to_owned(),
            ))
            .await?;

        rows.into_iter()
            .map(|row| {
                Ok(RoleRecord {
                    id: row.try_get("", "id")?,
                    code: row.try_get("", "code")?,
                    name: row.try_get("", "name")?,
                    description: row.try_get("", "description")?,
                })
            })
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

    /// 查询指定角色包含的权限代码列表。
    pub(crate) async fn list_role_permissions(&self, role_id: i64) -> Result<Vec<String>, DbErr> {
        let rows = self
            .database
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT DISTINCT auth_permissions.code AS code
                FROM auth_permissions
                INNER JOIN auth_role_permission_assignments
                    ON auth_role_permission_assignments.permission_id = auth_permissions.id
                WHERE auth_role_permission_assignments.role_id = ?
                ORDER BY auth_permissions.code
                "#,
                [role_id.into()],
            ))
            .await?;

        rows.into_iter()
            .map(|row| row.try_get("", "code"))
            .collect()
    }

    /// 按角色代码批量解析角色 ID；任一代码不存在时返回空。
    pub(crate) async fn find_role_ids_by_codes(
        &self,
        codes: &[String],
    ) -> Result<Option<Vec<i64>>, DbErr> {
        let mut role_ids = Vec::with_capacity(codes.len());
        for code in codes {
            let Some(role_id) = self.find_role_id_by_code(code).await? else {
                return Ok(None);
            };
            role_ids.push(role_id);
        }

        Ok(Some(role_ids))
    }

    /// 确保指定角色存在，并返回角色 ID。
    pub(crate) async fn ensure_role(
        &self,
        code: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<i64, DbErr> {
        // 角色定义属于 RBAC 内部结构，调用方只传业务语义，不拼接表结构。
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO auth_roles (code, name, description)
                VALUES (?, ?, ?)
                ON CONFLICT(code) DO NOTHING
                "#,
                [code.into(), name.into(), description.into()],
            ))
            .await?;

        self.find_role_id_by_code(code)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("auth role".to_owned()))
    }

    /// 确保指定权限存在，并返回权限 ID。
    pub(crate) async fn ensure_permission(
        &self,
        code: &str,
        description: Option<&str>,
    ) -> Result<i64, DbErr> {
        // 权限定义属于内置 RBAC 基础数据，已存在时不覆盖管理员后续调整。
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

    /// 给用户分配角色；已有分配保持不变。
    pub(crate) async fn assign_role_to_user(
        &self,
        user_id: i64,
        role_id: i64,
    ) -> Result<(), DbErr> {
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO auth_user_role_assignments (user_id, role_id)
                VALUES (?, ?)
                ON CONFLICT(user_id, role_id) DO NOTHING
                "#,
                [user_id.into(), role_id.into()],
            ))
            .await?;

        Ok(())
    }

    /// 整体替换用户角色；调用方应在事务内先完成最后管理员保护等业务校验。
    pub(crate) async fn replace_user_roles(
        &self,
        user_id: i64,
        role_ids: &[i64],
    ) -> Result<(), DbErr> {
        self.database
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "DELETE FROM auth_user_role_assignments WHERE user_id = ?",
                [user_id.into()],
            ))
            .await?;

        for role_id in role_ids {
            self.assign_role_to_user(user_id, *role_id).await?;
        }

        Ok(())
    }

    /// 判断除指定用户外是否还有 active admin，用于避免管理操作锁死系统。
    pub(crate) async fn has_other_active_admin(
        &self,
        excluded_user_id: i64,
    ) -> Result<bool, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"
                SELECT COUNT(DISTINCT auth_users.id) AS count
                FROM auth_users
                INNER JOIN auth_user_role_assignments
                    ON auth_user_role_assignments.user_id = auth_users.id
                INNER JOIN auth_roles
                    ON auth_roles.id = auth_user_role_assignments.role_id
                WHERE auth_users.status = 'active'
                    AND auth_users.id <> ?
                    AND auth_roles.code = 'admin'
                "#,
                [excluded_user_id.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("active admin count".to_owned()))?;

        let count: i64 = row.try_get("", "count")?;
        Ok(count > 0)
    }

    /// 给角色分配权限；已有分配保持不变。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn assign_permission_to_role(
        &self,
        role_id: i64,
        permission_id: i64,
    ) -> Result<(), DbErr> {
        assign_permission_to_role_on_connection(self.database, role_id, permission_id).await?;

        Ok(())
    }

    /// 同步角色权限；可选择保留旧权限或把角色权限替换为本次传入集合。
    pub(crate) async fn sync_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
        mode: RolePermissionSyncMode,
    ) -> Result<(), DbErr>
    where
        C: TransactionTrait,
    {
        // 修改角色权限必须在事务中完成，避免管理界面保存时出现短暂的半更新状态。
        let transaction = self.database.begin().await?;

        if mode == RolePermissionSyncMode::ReplaceExisting {
            transaction
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    "DELETE FROM auth_role_permission_assignments WHERE role_id = ?",
                    [role_id.into()],
                ))
                .await?;
        }

        for permission_id in permission_ids {
            assign_permission_to_role_on_connection(&transaction, role_id, *permission_id).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    /// 按角色代码查询角色 ID。
    async fn find_role_id_by_code(&self, code: &str) -> Result<Option<i64>, DbErr> {
        let row = self
            .database
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT id FROM auth_roles WHERE code = ?",
                [code.into()],
            ))
            .await?;

        row.map(|row| row.try_get("", "id")).transpose()
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

/// 在指定连接或事务上给角色追加权限；已有分配保持不变。
async fn assign_permission_to_role_on_connection<C>(
    connection: &C,
    role_id: i64,
    permission_id: i64,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO auth_role_permission_assignments (role_id, permission_id)
            VALUES (?, ?)
            ON CONFLICT(role_id, permission_id) DO NOTHING
            "#,
            [role_id.into(), permission_id.into()],
        ))
        .await?;

    Ok(())
}
