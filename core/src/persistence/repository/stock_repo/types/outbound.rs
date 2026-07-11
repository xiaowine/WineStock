//! 出库单仓储模型。
//!
//! 本模块属于 `core` 持久化层，描述出库事务的写入和读取边界。

use super::validate_positive_f64;
use crate::validation::{validate_not_blank, validate_optional_not_blank};

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
