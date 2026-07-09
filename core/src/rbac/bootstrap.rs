//! rbac 模块的权限定义初始化。
//!
//! 本模块属于 `rbac` 授权模型层，拥有内置权限的启动补齐。
//! 它不签发 JWT、不创建用户，也不处理平台配置或 HTTP token 解析。

use std::{error::Error, fmt};

use sea_orm::{DatabaseConnection, DbErr};

use crate::{
    persistence::repository::RbacRepository,
    stock::{
        AUDIT_READ_PERMISSION, STOCK_DASHBOARD_READ_PERMISSION, STOCK_INBOUND_APPROVE_PERMISSION,
        STOCK_INBOUND_CREATE_PERMISSION, STOCK_INBOUND_READ_PERMISSION,
        STOCK_ITEM_MANAGE_PERMISSION, STOCK_ITEM_READ_PERMISSION,
        STOCK_OUTBOUND_APPROVE_PERMISSION, STOCK_OUTBOUND_CREATE_PERMISSION,
        STOCK_OUTBOUND_READ_PERMISSION, STOCK_READ_PERMISSION, STOCK_SUBSTITUTE_MANAGE_PERMISSION,
        STOCK_SUBSTITUTE_READ_PERMISSION, STOCK_TEMPLATE_MANAGE_PERMISSION,
        STOCK_TEMPLATE_READ_PERMISSION, STOCK_WRITE_PERMISSION,
    },
    users::{
        READ_USER_PERMISSION, READ_USER_PERMISSION_DEFINITION_PERMISSION, REGISTER_USER_PERMISSION,
        RESET_USER_PASSWORD_PERMISSION, UPDATE_USER_PERMISSIONS_PERMISSION,
        UPDATE_USER_STATUS_PERMISSION,
    },
};

/// RBAC 基础定义初始化错误。
#[derive(Debug)]
pub enum RbacBootstrapError {
    /// SeaORM 或 SQLite 查询失败。
    Database(DbErr),
}

impl fmt::Display for RbacBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => write!(f, "failed to initialize built-in permissions"),
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

// 内置权限覆盖当前用户管理入口和首版库存业务能力。
const BUILTIN_PERMISSIONS: &[BuiltinPermission] = &[
    BuiltinPermission {
        code: AUDIT_READ_PERMISSION,
        description: "查询审计事件日志。",
    },
    BuiltinPermission {
        code: STOCK_INBOUND_APPROVE_PERMISSION,
        description: "审批或拒绝入库单。",
    },
    BuiltinPermission {
        code: STOCK_INBOUND_CREATE_PERMISSION,
        description: "创建入库单。",
    },
    BuiltinPermission {
        code: STOCK_INBOUND_READ_PERMISSION,
        description: "查看入库单列表、详情和入库历史筛选值。",
    },
    BuiltinPermission {
        code: STOCK_ITEM_MANAGE_PERMISSION,
        description: "创建、修改和软删除库存物品。",
    },
    BuiltinPermission {
        code: STOCK_ITEM_READ_PERMISSION,
        description: "查看库存物品列表、详情和物品筛选值。",
    },
    BuiltinPermission {
        code: STOCK_OUTBOUND_APPROVE_PERMISSION,
        description: "审批或拒绝出库单。",
    },
    BuiltinPermission {
        code: STOCK_OUTBOUND_CREATE_PERMISSION,
        description: "创建出库单。",
    },
    BuiltinPermission {
        code: STOCK_OUTBOUND_READ_PERMISSION,
        description: "查看出库单列表、详情和出库历史筛选值。",
    },
    BuiltinPermission {
        code: STOCK_READ_PERMISSION,
        description: "历史兼容的库存只读权限；具体查询接口使用细分权限。",
    },
    BuiltinPermission {
        code: STOCK_DASHBOARD_READ_PERMISSION,
        description: "查看库存看板总览和趋势。",
    },
    BuiltinPermission {
        code: STOCK_SUBSTITUTE_MANAGE_PERMISSION,
        description: "绑定或解绑替代料关系。",
    },
    BuiltinPermission {
        code: STOCK_SUBSTITUTE_READ_PERMISSION,
        description: "查看替代料关系。",
    },
    BuiltinPermission {
        code: STOCK_TEMPLATE_MANAGE_PERMISSION,
        description: "管理库存模板和模板字段。",
    },
    BuiltinPermission {
        code: STOCK_TEMPLATE_READ_PERMISSION,
        description: "查看库存模板列表和详情。",
    },
    BuiltinPermission {
        code: STOCK_WRITE_PERMISSION,
        description: "创建或修改库存数据。",
    },
    BuiltinPermission {
        code: READ_USER_PERMISSION_DEFINITION_PERMISSION,
        description: "查看权限定义。",
    },
    BuiltinPermission {
        code: UPDATE_USER_PERMISSIONS_PERMISSION,
        description: "整体替换用户权限。",
    },
    BuiltinPermission {
        code: RESET_USER_PASSWORD_PERMISSION,
        description: "直接重置用户密码。",
    },
    BuiltinPermission {
        code: READ_USER_PERMISSION,
        description: "查看用户列表和用户详情。",
    },
    BuiltinPermission {
        code: REGISTER_USER_PERMISSION,
        description: "注册新用户。",
    },
    BuiltinPermission {
        code: UPDATE_USER_STATUS_PERMISSION,
        description: "启用或停用用户账号。",
    },
];

/// 内置权限定义，启动时只补齐缺失记录，不覆盖已有记录。
#[derive(Debug, Clone, Copy)]
struct BuiltinPermission {
    /// 稳定权限代码，用于受保护接口授权。
    code: &'static str,

    /// 默认权限说明。
    description: &'static str,
}

/// 返回首个用户需要获得的全部内置权限代码。
pub(crate) fn builtin_permission_codes() -> Vec<String> {
    BUILTIN_PERMISSIONS
        .iter()
        .map(|permission| permission.code.to_owned())
        .collect()
}

/// 初始化内置权限定义；本函数不创建用户，也不覆盖已存在的权限文本。
pub(crate) async fn bootstrap_builtin_rbac(
    database: &DatabaseConnection,
) -> Result<(), RbacBootstrapError> {
    let rbac = RbacRepository::new(database);

    for permission in BUILTIN_PERMISSIONS {
        rbac.ensure_permission(permission.code, Some(permission.description))
            .await?;
    }

    Ok(())
}
