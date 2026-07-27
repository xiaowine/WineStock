//! 入库单仓储模型。
//!
//! 本模块属于 `core` 持久化层，描述入库事务的写入和读取边界。

use super::validate_positive_f64;
use crate::validation::{validate_not_blank, validate_optional_not_blank};

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
    #[garde(length(utf16, min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,
    /// 有效期日期文本。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,
}

/// 创建入库单的仓储输入。
#[derive(Debug, Clone, PartialEq, garde::Validate)]
pub(crate) struct CreateInboundOrder {
    /// 入库来源。
    #[garde(length(utf16, min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,
    /// 备注。
    #[garde(
        length(utf16, min = 1, max = 1024),
        custom(validate_optional_not_blank)
    )]
    pub notes: Option<String>,
    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,
    /// 直接入库时的审批人用户 ID；为空时创建 pending 单据。
    #[garde(skip)]
    pub approved_by_user_id: Option<i64>,
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
    /// 按入库单状态筛选。
    pub status: Option<String>,
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
    /// 查询时投影的当前物品名称。
    pub item_name: String,
    /// 查询时投影的当前物品编码。
    pub item_sku: String,
    /// 查询时投影的当前计量单位。
    pub item_unit: String,
    /// 查询时投影的当前物品主图文件 ID。
    pub item_image_file_id: i64,
    /// 入库数量。
    pub quantity: f64,
    /// 入库单价。
    pub unit_price: f64,
    /// 存储库位 ID。
    pub location_id: i64,
    /// 存储库位名称。
    pub location_name: String,
    /// 批次号。
    pub batch_no: Option<String>,
    /// 有效期。
    pub expires_at: Option<String>,
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
