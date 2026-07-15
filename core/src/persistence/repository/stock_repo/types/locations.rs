//! 库位和移库仓储模型。
//!
//! 本模块属于 `core` 持久化层，只描述库位子域的仓储边界数据。

use crate::validation::{validate_not_blank, validate_optional_not_blank};

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

/// 更新库位分组的仓储输入。
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
    /// 库位名称，未软删除库位内全局唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 可选库位备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
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
    /// 库位名称，未软删除库位内全局唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 可选库位备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,
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
    /// 库位名称。
    pub name: String,
    /// 可选库位备注。
    pub notes: Option<String>,
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
