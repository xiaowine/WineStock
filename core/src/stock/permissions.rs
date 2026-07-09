//! stock 库存业务权限常量。
//!
//! 本模块属于 `stock` 业务层，保存库存 API 使用的稳定权限代码。
//! 授权判断由 `security` 中间件执行，本模块不读取 token 或用户权限关系。

/// 历史兼容的库存只读权限代码；具体查询接口应使用更细分的领域只读权限。
pub(crate) const STOCK_READ_PERMISSION: &str = "stock.read";

/// 创建或修改库存数据的兼容性基础权限代码。
pub(crate) const STOCK_WRITE_PERMISSION: &str = "stock.write";

/// 查看库存物品列表、详情和物品筛选值的权限代码。
pub(crate) const STOCK_ITEM_READ_PERMISSION: &str = "stock.item.read";

/// 创建、修改和软删除库存物品的权限代码。
pub(crate) const STOCK_ITEM_MANAGE_PERMISSION: &str = "stock.item.manage";

/// 查看库存模板列表和详情的权限代码。
pub(crate) const STOCK_TEMPLATE_READ_PERMISSION: &str = "stock.template.read";

/// 管理库存模板和模板字段定义的权限代码。
pub(crate) const STOCK_TEMPLATE_MANAGE_PERMISSION: &str = "stock.template.manage";

/// 查看入库单列表、详情和入库历史筛选值的权限代码。
pub(crate) const STOCK_INBOUND_READ_PERMISSION: &str = "stock.inbound.read";

/// 创建入库单的权限代码；创建后单据保持待审批状态。
pub(crate) const STOCK_INBOUND_CREATE_PERMISSION: &str = "stock.inbound.create";

/// 审批或拒绝入库单的权限代码；审批通过后才增加库存。
pub(crate) const STOCK_INBOUND_APPROVE_PERMISSION: &str = "stock.inbound.approve";

/// 查看出库单列表、详情和出库历史筛选值的权限代码。
pub(crate) const STOCK_OUTBOUND_READ_PERMISSION: &str = "stock.outbound.read";

/// 创建出库单的权限代码；创建后单据保持待审批状态。
pub(crate) const STOCK_OUTBOUND_CREATE_PERMISSION: &str = "stock.outbound.create";

/// 审批或拒绝出库单的权限代码；审批通过后才扣减库存。
pub(crate) const STOCK_OUTBOUND_APPROVE_PERMISSION: &str = "stock.outbound.approve";

/// 查看库存看板总览和趋势的权限代码。
pub(crate) const STOCK_DASHBOARD_READ_PERMISSION: &str = "stock.dashboard.read";

/// 查看替代料关系的权限代码。
pub(crate) const STOCK_SUBSTITUTE_READ_PERMISSION: &str = "stock.substitute.read";

/// 整体替换或删除替代料关系的权限代码。
pub(crate) const STOCK_SUBSTITUTE_MANAGE_PERMISSION: &str = "stock.substitute.manage";

/// 查看库位分组树和库位列表的权限代码。
pub(crate) const STOCK_LOCATION_READ_PERMISSION: &str = "stock.location.read";

/// 管理库位分组、库位和整批次移库的权限代码。
pub(crate) const STOCK_LOCATION_MANAGE_PERMISSION: &str = "stock.location.manage";

/// 查询审计事件日志的权限代码。
pub(crate) const AUDIT_READ_PERMISSION: &str = "audit.read";
