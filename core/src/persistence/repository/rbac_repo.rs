//! rbac 模块 repository。
//!
//! 本模块属于 `core` 的持久化层，封装角色、权限、用户角色分配和角色权限分配。
//! 用户账号仓储不拥有 RBAC 表结构，鉴权和业务处理函数也不应直接拼接这些关联查询。

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionTrait,
};

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
