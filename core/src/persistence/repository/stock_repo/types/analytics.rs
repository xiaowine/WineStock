//! 看板、替代料和审计仓储模型。
//!
//! 本模块属于 `core` 持久化层，集中放置较小的只读聚合与审计边界模型。

use crate::validation::validate_optional_not_blank;

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
    /// 当前有库存但超过阈值天数未发生流水的物品。
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
    #[garde(
        length(utf16, min = 1, max = 1024),
        custom(validate_optional_not_blank)
    )]
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
    /// 替代料分类名称。
    pub substitute_item_category_name: Option<String>,
    /// 替代料主图文件 ID。
    pub substitute_item_image_file_id: i64,
    /// 替代料计量单位。
    pub substitute_item_unit: String,
    /// 替代料再订货点。
    pub substitute_item_reorder_point: Option<f64>,
    /// 替代料当前库存量。
    pub quantity: f64,
    /// 替代料服务端计算的库存状态代码。
    pub substitute_item_stock_state: String,
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
