//! 库存仓储输入和读取模型。
//!
//! 本模块属于 `core` 持久化层的 stock repository，集中定义仓储边界使用的数据结构。
//! 它不执行数据库查询，也不拥有 HTTP DTO。

use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::persistence::entity::{stock_template, stock_template_field};

/// 创建库存物品的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateStockItem {
    /// 物品名称，裁剪后不能为空。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU，未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID；为空表示暂不关联模板。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位，裁剪后不能为空。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,
}

/// 更新库存物品的仓储输入；为空字段表示不修改。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct UpdateStockItem {
    /// 物品名称，存在时裁剪后不能为空。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 物品 SKU，存在时裁剪后不能为空且未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub sku: Option<String>,

    /// 关联模板 ID；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub category_id: Option<Option<i64>>,

    /// 计量单位，存在时裁剪后不能为空。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,

    /// 物品描述；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,

    /// 参考单价；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub default_price: Option<Option<f64>>,

    /// 再订货点；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub reorder_point: Option<Option<f64>>,
}

/// 库存物品分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListStockItems {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量，服务层负责限制最大值。
    pub page_size: u64,

    /// 物品、模板和当前库存扩展属性模糊搜索关键字。
    pub search: Option<String>,

    /// 按模板 ID 筛选。
    pub category_id: Option<i64>,
}

/// 创建库位分组的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateLocationGroup {
    /// 上级分组 ID；为空表示根分组。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称，同一上级分组内不能重复。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 分组排序值，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 更新库位分组的仓储输入；`parent_id` 为空表示移动到根分组。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateLocationGroup {
    /// 上级分组 ID；为空表示根分组。
    #[garde(skip)]
    pub parent_id: Option<i64>,

    /// 分组名称，同一上级分组内不能重复。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 分组排序值，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 创建库位的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateLocation {
    /// 所属库位分组 ID。
    #[garde(range(min = 1))]
    pub group_id: i64,

    /// 库位编码，未软删除库位内全局唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub code: String,

    /// 库位名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 库位排序值，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 更新库位的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateLocation {
    /// 所属库位分组 ID。
    #[garde(range(min = 1))]
    pub group_id: i64,

    /// 库位编码，未软删除库位内全局唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub code: String,

    /// 库位名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 库位排序值，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 整批次移库的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateLocationTransfer {
    /// 需要移动的库存批次 ID。
    #[garde(range(min = 1))]
    pub batch_id: i64,

    /// 调用方确认的原库位 ID，用于避免基于过期页面误移库。
    #[garde(range(min = 1))]
    pub from_location_id: i64,

    /// 目标库位 ID。
    #[garde(range(min = 1))]
    pub to_location_id: i64,

    /// 移库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 操作人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,
}

/// 库位分组读取模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockLocationGroupRecord {
    /// 分组 ID。
    pub id: i64,

    /// 上级分组 ID。
    pub parent_id: Option<i64>,

    /// 分组名称。
    pub name: String,

    /// 排序值。
    pub sort_order: i32,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,
}

/// 库位读取模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockLocationRecord {
    /// 库位 ID。
    pub id: i64,

    /// 所属分组 ID。
    pub group_id: i64,

    /// 所属分组名称。
    pub group_name: String,

    /// 库位编码。
    pub code: String,

    /// 库位名称。
    pub name: String,

    /// 排序值。
    pub sort_order: i32,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,
}

/// 整批次移库读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockLocationTransferRecord {
    /// 移库记录 ID。
    pub id: i64,

    /// 被移动批次 ID。
    pub batch_id: i64,

    /// 被移动物品 ID。
    pub item_id: i64,

    /// 原库位 ID。
    pub from_location_id: i64,

    /// 目标库位 ID。
    pub to_location_id: i64,

    /// 本次移动的当前批次余额。
    pub quantity: f64,

    /// 移库备注。
    pub notes: Option<String>,

    /// 操作人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 移库时间。
    pub created_at: String,
}

/// 创建模板字段定义的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct TemplateFieldInput {
    /// 字段名称，同一模板内不能重复。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型稳定代码。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub field_type: String,

    /// 是否必填。
    #[garde(skip)]
    pub required: bool,

    /// 是否可用于搜索。
    #[garde(skip)]
    pub searchable: bool,

    /// 候选值 JSON，仅 `select` 字段使用。
    #[garde(length(min = 1, max = 4096), custom(validate_optional_not_blank))]
    pub options_json: Option<String>,

    /// 默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,

    /// 字段排序，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 创建库存模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateStockTemplate {
    /// 模板名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldInput>,
}

/// 更新库存模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateStockTemplate {
    /// 模板名称，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 模板说明；外层 Option 表示是否修改，内层 Option 表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,

    /// 模板字段定义；存在时整体替换旧字段。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldInput>>,
}

/// 库存模板详情，包含模板基础资料和字段定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StockTemplateDetail {
    /// 模板基础资料。
    pub template: stock_template::Model,

    /// 模板字段定义，按 `sort_order, id` 排序。
    pub fields: Vec<stock_template_field::Model>,
}

/// 库存物品详情读取模型，包含基础资料和当前有效批次聚合。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemDetail {
    /// 物品基础资料。
    pub item: crate::persistence::entity::stock_item::Model,

    /// 当前剩余库存总量，只统计 `remaining_quantity > 0` 的批次。
    pub current_quantity: f64,

    /// 当前库存价值，按批次剩余数量乘以批次单价汇总。
    pub inventory_value: f64,

    /// 当前库存按库位聚合后的分布。
    pub locations: Vec<StockItemLocationRecord>,

    /// 当前仍有余额的批次摘要。
    pub batches: Vec<StockItemBatchRecord>,
}

/// 物品当前库存库位聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemLocationRecord {
    /// 库位 ID。
    pub location_id: i64,

    /// 库位编码。
    pub location_code: String,

    /// 库位名称。
    pub location_name: String,

    /// 该库位当前剩余库存量。
    pub quantity: f64,

    /// 该库位当前库存价值。
    pub value: f64,

    /// 该库位当前仍有余额的批次数。
    pub batch_count: i64,
}

/// 物品当前库存批次摘要读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockItemBatchRecord {
    /// 批次 ID。
    pub id: i64,

    /// 批次号。
    pub batch_no: String,

    /// 批次库位 ID。
    pub location_id: i64,

    /// 批次库位编码。
    pub location_code: String,

    /// 批次库位名称。
    pub location_name: String,

    /// 入库时的初始数量。
    pub initial_quantity: f64,

    /// 当前剩余数量。
    pub remaining_quantity: f64,

    /// 批次单价。
    pub unit_cost: f64,

    /// 当前批次库存价值。
    pub value: f64,

    /// 入库审批时间。
    pub received_at: String,

    /// 有效期。
    pub expires_at: Option<String>,
}

/// 创建入库单明细的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateInboundOrderItem {
    /// 入库物品 ID。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 入库数量，必须大于 0。
    #[garde(custom(validate_positive_f64))]
    pub quantity: f64,

    /// 入库单价，不允许为负。
    #[garde(range(min = 0.0))]
    pub unit_price: f64,

    /// 存储库位 ID。
    #[garde(range(min = 1))]
    pub location_id: i64,

    /// 外部批次号；为空时审批阶段生成内部批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期字符串；首版仅保存调用方传入的日期文本。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 模板扩展属性 JSON 字符串。
    #[garde(length(min = 1, max = 8192), custom(validate_optional_not_blank))]
    pub ext_attributes_json: Option<String>,
}

/// 创建入库单的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateInboundOrder {
    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 入库明细。
    #[garde(dive)]
    pub items: Vec<CreateInboundOrderItem>,
}

/// 入库单分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListInboundOrders {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点。
    pub date_from: Option<String>,

    /// 创建时间终点。
    pub date_to: Option<String>,

    /// 入库历史自由搜索关键字。
    pub search: Option<String>,
}

/// 入库单主表读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderRecord {
    /// 入库单 ID。
    pub id: i64,

    /// 入库来源。
    pub source: String,

    /// 单据状态。
    pub status: String,

    /// 备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,

    /// 审批时间。
    pub approved_at: Option<String>,

    /// 拒绝时间。
    pub rejected_at: Option<String>,
}

/// 入库单明细读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderItemRecord {
    /// 明细 ID。
    pub id: i64,

    /// 所属入库单 ID。
    pub order_id: i64,

    /// 物品 ID。
    pub item_id: i64,

    /// 入库数量。
    pub quantity: f64,

    /// 入库单价。
    pub unit_price: f64,

    /// 存储库位 ID。
    pub location_id: i64,

    /// 存储库位编码。
    pub location_code: String,

    /// 存储库位名称。
    pub location_name: String,

    /// 批次号。
    pub batch_no: Option<String>,

    /// 有效期。
    pub expires_at: Option<String>,

    /// 模板扩展属性 JSON。
    pub ext_attributes_json: Option<String>,

    /// 创建时间。
    pub created_at: String,
}

/// 入库单详情读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InboundOrderDetail {
    /// 入库单主表记录。
    pub order: InboundOrderRecord,

    /// 入库单明细。
    pub items: Vec<InboundOrderItemRecord>,
}

/// 创建出库单明细的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateOutboundOrderItem {
    /// 出库物品 ID。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 出库数量，必须大于 0。
    #[garde(custom(validate_positive_f64))]
    pub quantity: f64,

    /// 指定扣减批次；为空时审批阶段按 FIFO 扣减。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位 ID；为空时审批阶段按全部当前库存 FIFO 扣减。
    #[garde(skip)]
    pub location_id: Option<i64>,
}

/// 创建出库单的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateOutboundOrder {
    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 出库明细。
    #[garde(dive)]
    pub items: Vec<CreateOutboundOrderItem>,
}

/// 出库单分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListOutboundOrders {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点。
    pub date_from: Option<String>,

    /// 创建时间终点。
    pub date_to: Option<String>,

    /// 出库历史自由搜索关键字。
    pub search: Option<String>,
}

/// 出库单主表读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderRecord {
    /// 出库单 ID。
    pub id: i64,

    /// 出库去向。
    pub destination: String,

    /// 单据状态。
    pub status: String,

    /// 备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,

    /// 更新时间。
    pub updated_at: String,

    /// 审批时间。
    pub approved_at: Option<String>,

    /// 拒绝时间。
    pub rejected_at: Option<String>,
}

/// 出库单明细读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderItemRecord {
    /// 明细 ID。
    pub id: i64,

    /// 所属出库单 ID。
    pub order_id: i64,

    /// 物品 ID。
    pub item_id: i64,

    /// 出库数量。
    pub quantity: f64,

    /// 指定扣减批次。
    pub batch_id: Option<i64>,

    /// 出库库位 ID。
    pub location_id: Option<i64>,

    /// 出库库位编码。
    pub location_code: Option<String>,

    /// 出库库位名称。
    pub location_name: Option<String>,

    /// 创建时间。
    pub created_at: String,
}

/// 出库单详情读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutboundOrderDetail {
    /// 出库单主表记录。
    pub order: OutboundOrderRecord,

    /// 出库单明细。
    pub items: Vec<OutboundOrderItemRecord>,
}

/// 看板总览聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DashboardOverviewRecord {
    /// 未软删除的库存物品种类数。
    pub total_items: i64,

    /// 当前所有有效批次的剩余总数量。
    pub total_quantity: f64,

    /// 当前所有有效批次按批次成本计算的库存总价值。
    pub total_value: f64,

    /// 最近三天已审批入库流水总数量。
    pub inbound_3d: f64,

    /// 最近三天已审批出库流水总数量。
    pub outbound_3d: f64,

    /// 当前有库存但超过阈值天数未发生出入库流水的物品。
    pub slow_moving_items: Vec<SlowMovingStockItemRecord>,
}

/// 呆滞料聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlowMovingStockItemRecord {
    /// 物品 ID。
    pub item_id: i64,

    /// 物品名称。
    pub item_name: String,

    /// 当前剩余库存量。
    pub quantity: f64,

    /// 当前库存价值。
    pub value: f64,

    /// 最近一次出入库流水距今天数。
    pub days_since_last_movement: i64,
}

/// 每日出入库趋势聚合读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DailyMovementTrendRecord {
    /// 日期，格式为 `YYYY-MM-DD`。
    pub date: String,

    /// 当日已审批入库数量。
    pub inbound_quantity: f64,

    /// 当日已审批出库数量。
    pub outbound_quantity: f64,
}

/// 替代料关系写库输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct StockSubstituteInput {
    /// 替代料物品 ID。
    #[garde(range(min = 1))]
    pub substitute_item_id: i64,

    /// 替代优先级，数值越小越优先。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
}

/// 替代料关系读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StockSubstituteRecord {
    /// 主物品 ID。
    pub item_id: i64,

    /// 主物品名称。
    pub item_name: String,

    /// 主物品 SKU。
    pub item_sku: String,

    /// 替代料物品 ID。
    pub substitute_item_id: i64,

    /// 替代料物品名称。
    pub substitute_item_name: String,

    /// 替代料物品 SKU。
    pub substitute_item_sku: String,

    /// 替代料当前库存量。
    pub quantity: f64,

    /// 替代优先级。
    pub priority: i32,

    /// 兼容性备注。
    pub notes: Option<String>,

    /// 创建人用户 ID。
    pub created_by_user_id: Option<i64>,

    /// 创建时间。
    pub created_at: String,
}

/// 审计事件分页查询条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListAuditEvents {
    /// 页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 按实体类型筛选。
    pub entity_type: Option<String>,

    /// 按实体 ID 筛选。
    pub entity_id: Option<i64>,

    /// 按动作筛选。
    pub action: Option<String>,

    /// 按操作人用户 ID 筛选。
    pub user_id: Option<i64>,

    /// 操作时间起点。
    pub date_from: Option<String>,

    /// 操作时间终点。
    pub date_to: Option<String>,
}

/// 审计事件读取模型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AuditEventRecord {
    /// 审计事件 ID。
    pub id: i64,

    /// 操作时间。
    pub timestamp: String,

    /// 操作人用户 ID。
    pub user_id: Option<i64>,

    /// 操作人用户名。
    pub username: Option<String>,

    /// 被操作实体类型。
    pub entity_type: String,

    /// 被操作实体 ID。
    pub entity_id: Option<i64>,

    /// 操作动作。
    pub action: String,

    /// 事件详情 JSON 字符串。
    pub details_json: Option<String>,
}
pub(crate) struct Page<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 满足条件的总记录数。
    pub total: u64,
}

fn validate_positive_f64(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
    }
}
