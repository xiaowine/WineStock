//! stock 库存业务权限常量。
//!
//! 本模块属于 `stock` 业务层，保存库存 API 使用的稳定权限代码。
//! 授权判断由 `security` 中间件执行，本模块不读取 token 或用户权限关系。

/// 查看库存数据、库存列表、看板和替代料只读信息的权限代码。
pub(crate) const STOCK_READ_PERMISSION: &str = "stock.read";

/// 创建或修改库存数据的兼容性基础权限代码。
pub(crate) const STOCK_WRITE_PERMISSION: &str = "stock.write";

/// 创建、修改和软删除库存物品的权限代码。
pub(crate) const STOCK_ITEM_MANAGE_PERMISSION: &str = "stock.item.manage";

/// 管理库存模板和模板字段定义的权限代码。
pub(crate) const STOCK_TEMPLATE_MANAGE_PERMISSION: &str = "stock.template.manage";

/// 创建入库单的权限代码；创建后单据保持待审批状态。
pub(crate) const STOCK_INBOUND_CREATE_PERMISSION: &str = "stock.inbound.create";

/// 审批或拒绝入库单的权限代码；审批通过后才增加库存。
pub(crate) const STOCK_INBOUND_APPROVE_PERMISSION: &str = "stock.inbound.approve";

/// 创建出库单的权限代码；创建后单据保持待审批状态。
pub(crate) const STOCK_OUTBOUND_CREATE_PERMISSION: &str = "stock.outbound.create";

/// 审批或拒绝出库单的权限代码；审批通过后才扣减库存。
pub(crate) const STOCK_OUTBOUND_APPROVE_PERMISSION: &str = "stock.outbound.approve";

/// 绑定或解绑替代料关系的权限代码。
pub(crate) const STOCK_SUBSTITUTE_MANAGE_PERMISSION: &str = "stock.substitute.manage";

/// 查询审计事件日志的权限代码。
pub(crate) const AUDIT_READ_PERMISSION: &str = "audit.read";
