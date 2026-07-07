//! rbac 模块的基础定义初始化。
//!
//! 本模块属于 `rbac` 授权模型层，拥有内置角色、权限和角色权限关系的启动补齐。
//! 它不签发 JWT、不创建用户，也不处理平台配置或 HTTP token 解析。

use std::{error::Error, fmt};

use sea_orm::{DatabaseConnection, DbErr};

use crate::{
    persistence::repository::{RbacRepository, RolePermissionSyncMode},
    users::{MANAGE_USER_PERMISSION, REGISTER_USER_PERMISSION},
};

use super::policy::{ADMIN_ROLE_CODE, ADMIN_ROLE_NAME, STAFF_ROLE_CODE, STAFF_ROLE_NAME, STOCK_READ_PERMISSION, STOCK_WRITE_PERMISSION, VIEWER_ROLE_CODE, VIEWER_ROLE_NAME};

/// RBAC 基础定义初始化错误。
#[derive(Debug)]
pub enum RbacBootstrapError {
    /// SeaORM 或 SQLite 查询失败。
    Database(DbErr),
}

impl fmt::Display for RbacBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => write!(f, "failed to initialize built-in RBAC definitions"),
        }
    }
}

impl Error for RbacBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
        }
    }
}

impl From<DbErr> for RbacBootstrapError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

// 内置角色只定义系统起步所需的稳定代码，用户仍由注册接口显式创建。
const BUILTIN_ROLES: &[BuiltinRole] = &[
    BuiltinRole {
        code: ADMIN_ROLE_CODE,
        name: ADMIN_ROLE_NAME,
        description: Some("系统管理员，拥有全部内置权限。"),
    },
    BuiltinRole {
        code: STAFF_ROLE_CODE,
        name: STAFF_ROLE_NAME,
        description: Some("日常业务操作用户。"),
    },
    BuiltinRole {
        code: VIEWER_ROLE_CODE,
        name: VIEWER_ROLE_NAME,
        description: Some("只读访问用户。"),
    },
];

// 内置权限覆盖当前用户管理入口和库存域的基础读写能力。
const BUILTIN_PERMISSIONS: &[BuiltinPermission] = &[
    BuiltinPermission {
        code: REGISTER_USER_PERMISSION,
        description: "注册新用户。",
    },
    BuiltinPermission {
        code: MANAGE_USER_PERMISSION,
        description: "管理用户、角色和权限。",
    },
    BuiltinPermission {
        code: STOCK_READ_PERMISSION,
        description: "查看库存数据。",
    },
    BuiltinPermission {
        code: STOCK_WRITE_PERMISSION,
        description: "创建或修改库存数据。",
    },
];

const BUILTIN_ROLE_PERMISSIONS: &[(&str, &[&str])] = &[
    (
        ADMIN_ROLE_CODE,
        &[
            REGISTER_USER_PERMISSION,
            MANAGE_USER_PERMISSION,
            STOCK_READ_PERMISSION,
            STOCK_WRITE_PERMISSION,
        ],
    ),
    (
        STAFF_ROLE_CODE,
        &[STOCK_READ_PERMISSION, STOCK_WRITE_PERMISSION],
    ),
    (VIEWER_ROLE_CODE, &[STOCK_READ_PERMISSION]),
];

/// 内置角色定义，启动时只补齐缺失记录，不覆盖已有记录。
#[derive(Debug, Clone, Copy)]
struct BuiltinRole {
    /// 稳定角色代码，用于 JWT claims、角色展示和分配关系。
    code: &'static str,

    /// 默认角色名称。
    name: &'static str,

    /// 默认角色说明。
    description: Option<&'static str>,
}

/// 内置权限定义，启动时只补齐缺失记录，不覆盖已有记录。
#[derive(Debug, Clone, Copy)]
struct BuiltinPermission {
    /// 稳定权限代码，用于受保护接口授权。
    code: &'static str,

    /// 默认权限说明。
    description: &'static str,
}

/// 初始化内置 RBAC 定义；本函数不创建用户，也不覆盖已存在的角色/权限文本。
pub(crate) async fn bootstrap_builtin_rbac(
    database: &DatabaseConnection,
) -> Result<(), RbacBootstrapError> {
    let rbac = RbacRepository::new(database);

    for role in BUILTIN_ROLES {
        rbac.ensure_role(role.code, role.name, role.description)
            .await?;
    }

    for permission in BUILTIN_PERMISSIONS {
        rbac.ensure_permission(permission.code, Some(permission.description))
            .await?;
    }

    for (role_code, permission_codes) in BUILTIN_ROLE_PERMISSIONS {
        let role_id = rbac.ensure_role(role_code, role_code, None).await?;
        let mut permission_ids = Vec::with_capacity(permission_codes.len());
        for permission_code in *permission_codes {
            let permission_id = rbac.ensure_permission(permission_code, None).await?;
            permission_ids.push(permission_id);
        }
        rbac.sync_role_permissions(
            role_id,
            &permission_ids,
            RolePermissionSyncMode::PreserveExisting,
        )
        .await?;
    }

    Ok(())
}
