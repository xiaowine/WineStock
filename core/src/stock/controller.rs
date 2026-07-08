//! stock 模块 HTTP 控制器。
//!
//! 本模块属于 `stock` 业务层，负责库存 API 的请求/响应 DTO、Axum handler 和 OpenAPI 标注。
//! 具体业务流程交给 `service`，本模块不直接访问数据库。

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use winestock_shared::validation::{validate_not_blank, validate_optional_not_blank};

use crate::{http::ValidatedJson, security::CurrentUser, state::CoreState};

use super::service::{self, PaginatedResponse, StockApiError};

/// 模板字段类型。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TemplateFieldType {
    /// 普通文本字段。
    Text,

    /// 数值字段。
    Number,

    /// 预置选项字段。
    Select,

    /// 日期字段，值使用日期字符串。
    Date,

    /// 文件字段，值引用文件元数据。
    File,

    /// 布尔字段，默认值只允许 `true` 或 `false`。
    Boolean,
}

impl TemplateFieldType {
    /// 返回数据库中保存的稳定字段类型代码。
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Select => "select",
            Self::Date => "date",
            Self::File => "file",
            Self::Boolean => "boolean",
        }
    }

    /// 从数据库字段类型代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "select" => Ok(Self::Select),
            "date" => Ok(Self::Date),
            "file" => Ok(Self::File),
            "boolean" => Ok(Self::Boolean),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 模板字段定义请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateFieldDef {
    /// 字段名称，同一模板内必须唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,

    /// 是否必填；未传时默认为 false。
    #[garde(skip)]
    pub required: Option<bool>,

    /// 是否可用于搜索；未传时默认为 false。
    #[garde(skip)]
    pub searchable: Option<bool>,

    /// `select` 字段的候选值；其他字段类型不允许传入。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,

    /// 默认值；数值、布尔和选项字段会执行额外业务校验。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
}

/// 创建库存模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCreateRequest {
    /// 模板名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义列表，至少一个字段。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldDef>,
}

/// 更新库存模板请求；字段为空表示不修改。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateUpdateRequest {
    /// 模板名称，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 模板说明；当前首版接口不通过 null 清空该字段。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义；存在时整体替换旧字段。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldDef>>,
}

/// 复制模板请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCopyRequest {
    /// 新模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
}

/// 模板字段响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct TemplateFieldResponse {
    /// 字段 ID。
    #[garde(skip)]
    pub id: i64,

    /// 字段名称。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,

    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,

    /// 是否必填。
    #[garde(skip)]
    pub required: bool,

    /// 是否可用于搜索。
    #[garde(skip)]
    pub searchable: bool,

    /// `select` 字段的候选值。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,

    /// 默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,

    /// 字段排序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 库存模板响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct TemplateResponse {
    /// 模板 ID。
    #[garde(skip)]
    pub id: i64,

    /// 模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 模板字段定义。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldResponse>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,
}

/// 创建库存物品请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemCreateRequest {
    /// 物品名称，服务端会裁剪首尾空白。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU，未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
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

/// 更新库存物品请求；字段为空表示不修改。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemUpdateRequest {
    /// 物品名称，存在时服务端会裁剪首尾空白。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 物品 SKU，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub sku: Option<String>,

    /// 关联模板 ID；当前首版接口不通过 null 清空该字段。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,

    /// 物品描述；当前首版接口不通过 null 清空该字段。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,
}

/// 库存物品分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按名称或 SKU 模糊搜索。
    pub search: Option<String>,

    /// 按关联模板 ID 筛选。
    pub category_id: Option<i64>,
}

/// 库存物品响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemResponse {
    /// 物品 ID。
    #[garde(skip)]
    pub id: i64,

    /// 物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联模板 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,

    /// 参考单价。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,
}

/// 入库单状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OrderStatus {
    /// 待审批，尚未改变库存。
    Pending,

    /// 已审批，审批事务已写入批次、库存流水和审计事件。
    Approved,

    /// 已拒绝，不能再审批。
    Rejected,
}

impl OrderStatus {
    /// 从数据库状态代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 创建入库单明细请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundItemRequest {
    /// 入库物品 ID，必须指向未软删除物品。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 入库数量，必须大于 0。
    #[garde(custom(validate_positive_number))]
    pub quantity: f64,

    /// 入库单价，不允许为负。
    #[garde(skip)]
    pub unit_price: f64,

    /// 入库库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,

    /// 外部批次号；为空时审批阶段生成内部批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期文本，首版按调用方输入保存。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 模板扩展属性；审批阶段按物品关联模板校验。
    #[garde(skip)]
    pub ext_attributes: Option<Value>,
}

/// 创建入库单请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct InboundCreateRequest {
    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 入库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 入库明细，至少一条。
    #[garde(dive)]
    pub items: Vec<InboundItemRequest>,
}

/// 入库单分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct InboundListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点，使用 SQLite UTC 字符串格式。
    pub date_from: Option<String>,

    /// 创建时间终点，使用 SQLite UTC 字符串格式。
    pub date_to: Option<String>,
}

/// 入库单明细响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct InboundItemResponse {
    /// 明细 ID。
    #[garde(skip)]
    pub id: i64,

    /// 所属入库单 ID。
    #[garde(skip)]
    pub order_id: i64,

    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 入库数量。
    #[garde(skip)]
    pub quantity: f64,

    /// 入库单价。
    #[garde(skip)]
    pub unit_price: f64,

    /// 入库库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,

    /// 批次号。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub batch_no: Option<String>,

    /// 有效期文本。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub expires_at: Option<String>,

    /// 模板扩展属性。
    #[garde(skip)]
    pub ext_attributes: Option<Value>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 入库单响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct InboundResponse {
    /// 入库单 ID。
    #[garde(skip)]
    pub id: i64,

    /// 入库来源。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub source: String,

    /// 入库状态。
    #[garde(skip)]
    pub status: OrderStatus,

    /// 入库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    #[garde(skip)]
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    #[garde(skip)]
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,

    /// 审批时间。
    #[garde(skip)]
    pub approved_at: Option<String>,

    /// 拒绝时间。
    #[garde(skip)]
    pub rejected_at: Option<String>,

    /// 入库明细。
    #[garde(dive)]
    pub items: Vec<InboundItemResponse>,
}

/// 创建出库单明细请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutboundItemRequest {
    /// 出库物品 ID，必须指向未软删除物品。
    #[garde(range(min = 1))]
    pub item_id: i64,

    /// 出库数量，必须大于 0。
    #[garde(custom(validate_positive_number))]
    pub quantity: f64,

    /// 指定扣减批次；为空时审批阶段按 FIFO 扣减。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,
}

/// 创建出库单请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutboundCreateRequest {
    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 出库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 出库明细，至少一条。
    #[garde(dive)]
    pub items: Vec<OutboundItemRequest>,
}

/// 出库单分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct OutboundListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按物品 ID 筛选。
    pub item_id: Option<i64>,

    /// 创建时间起点，使用 SQLite UTC 字符串格式。
    pub date_from: Option<String>,

    /// 创建时间终点，使用 SQLite UTC 字符串格式。
    pub date_to: Option<String>,
}

/// 出库单明细响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct OutboundItemResponse {
    /// 明细 ID。
    #[garde(skip)]
    pub id: i64,

    /// 所属出库单 ID。
    #[garde(skip)]
    pub order_id: i64,

    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 出库数量。
    #[garde(skip)]
    pub quantity: f64,

    /// 指定扣减批次。
    #[garde(skip)]
    pub batch_id: Option<i64>,

    /// 出库库位。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub location: Option<String>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 出库单响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct OutboundResponse {
    /// 出库单 ID。
    #[garde(skip)]
    pub id: i64,

    /// 出库去向。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub destination: String,

    /// 出库状态。
    #[garde(skip)]
    pub status: OrderStatus,

    /// 出库备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 审批人用户 ID。
    #[garde(skip)]
    pub approved_by_user_id: Option<i64>,

    /// 拒绝人用户 ID。
    #[garde(skip)]
    pub rejected_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,

    /// 审批时间。
    #[garde(skip)]
    pub approved_at: Option<String>,

    /// 拒绝时间。
    #[garde(skip)]
    pub rejected_at: Option<String>,

    /// 出库明细。
    #[garde(dive)]
    pub items: Vec<OutboundItemResponse>,
}

/// 呆滞料看板条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct SlowMovingItem {
    /// 物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub item_name: String,

    /// 当前库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 当前库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 最近一次出入库流水距今天数。
    #[garde(skip)]
    pub days_since_last_movement: i64,
}

/// 库存看板总览响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct DashboardOverviewResponse {
    /// 未软删除的库存物品种类数。
    #[garde(skip)]
    pub total_items: i64,

    /// 当前库存总数量。
    #[garde(skip)]
    pub total_quantity: f64,

    /// 当前库存总价值。
    #[garde(skip)]
    pub total_value: f64,

    /// 最近三天入库数量。
    #[garde(skip)]
    pub inbound_3d: f64,

    /// 最近三天出库数量。
    #[garde(skip)]
    pub outbound_3d: f64,

    /// 当前呆滞料列表。
    #[garde(dive)]
    pub slow_moving_items: Vec<SlowMovingItem>,
}

/// 看板趋势查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct TrendsQuery {
    /// 趋势天数，默认 30，最大 365；小于 1 时按 1 处理。
    pub days: Option<u64>,
}

/// 每日出入库趋势响应条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct DailyTrend {
    /// 日期，格式为 `YYYY-MM-DD`。
    #[garde(length(equal = 10))]
    pub date: String,

    /// 当日入库数量。
    #[garde(skip)]
    pub inbound_quantity: f64,

    /// 当日出库数量。
    #[garde(skip)]
    pub outbound_quantity: f64,
}

/// 看板趋势响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct TrendsResponse {
    /// 按日期升序排列的趋势数据。
    #[garde(dive)]
    pub daily: Vec<DailyTrend>,
}

/// 替代料绑定条目。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct SubstituteItem {
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

/// 替代料绑定请求；提交列表会整体替换当前物品的替代料关系。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubstituteBindRequest {
    /// 替代料列表；允许空列表，用于清空当前物品的所有替代料关系。
    #[garde(dive)]
    pub substitutes: Vec<SubstituteItem>,
}

/// 替代料详情响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct SubstituteDetailResponse {
    /// 主物品 ID。
    #[garde(skip)]
    pub item_id: i64,

    /// 替代料物品 ID。
    #[garde(skip)]
    pub substitute_item_id: i64,

    /// 替代料物品名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub substitute_item_name: String,

    /// 替代料当前库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 替代优先级。
    #[garde(range(min = 1))]
    pub priority: i32,

    /// 兼容性备注。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub notes: Option<String>,

    /// 创建人用户 ID。
    #[garde(skip)]
    pub created_by_user_id: Option<i64>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,
}

/// 事件日志分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct EventListQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按实体类型筛选。
    pub entity_type: Option<String>,

    /// 按实体 ID 筛选。
    pub entity_id: Option<i64>,

    /// 按操作动作筛选。
    pub action: Option<String>,

    /// 按操作人用户 ID 筛选。
    pub user_id: Option<i64>,

    /// 操作时间起点，使用 SQLite UTC 字符串格式。
    pub date_from: Option<String>,

    /// 操作时间终点，使用 SQLite UTC 字符串格式。
    pub date_to: Option<String>,
}

/// 事件日志响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct EventLogResponse {
    /// 事件 ID。
    #[garde(skip)]
    pub id: i64,

    /// 操作时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub timestamp: String,

    /// 操作人用户 ID。
    #[garde(skip)]
    pub user_id: Option<i64>,

    /// 操作人用户名；用户被删除或外键置空时为空。
    #[garde(length(min = 1, max = 64), custom(validate_optional_not_blank))]
    pub username: Option<String>,

    /// 被操作实体类型。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub entity_type: String,

    /// 被操作实体 ID。
    #[garde(skip)]
    pub entity_id: Option<i64>,

    /// 操作动作。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub action: String,

    /// 事件详情 JSON。
    #[garde(skip)]
    pub details: Value,
}

#[utoipa::path(
    post,
    path = "/api/templates",
    tag = "stock",
    request_body = TemplateCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Template created", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 创建库存模板。
pub(crate) async fn create_template(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<TemplateCreateRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_template(&state, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "stock",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template list", body = Vec<TemplateResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 查询库存模板列表。
pub(crate) async fn list_templates(
    State(state): State<CoreState>,
) -> Result<Json<Vec<TemplateResponse>>, StockApiError> {
    Ok(Json(service::list_templates(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/templates/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template detail", body = TemplateResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Template not found", body = String)
    )
)]
/// 查询单个库存模板。
pub(crate) async fn get_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<TemplateResponse>, StockApiError> {
    Ok(Json(service::get_template(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/templates/{id}",
    tag = "stock",
    request_body = TemplateUpdateRequest,
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Template updated", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 更新库存模板。
pub(crate) async fn update_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<TemplateUpdateRequest>,
) -> Result<Json<TemplateResponse>, StockApiError> {
    Ok(Json(service::update_template(&state, id, request).await?))
}

#[utoipa::path(
    delete,
    path = "/api/templates/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template is referenced by active items", body = String)
    )
)]
/// 软删除库存模板。
pub(crate) async fn delete_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_template(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/templates/{id}/copy",
    tag = "stock",
    request_body = TemplateCopyRequest,
    params(("id" = i64, Path, description = "Template ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Template copied", body = TemplateResponse),
        (status = 400, description = "Invalid template request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Template manage permission required", body = String),
        (status = 404, description = "Template not found", body = String),
        (status = 409, description = "Template name already exists", body = String)
    )
)]
/// 复制库存模板。
pub(crate) async fn copy_template(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<TemplateCopyRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::copy_template(&state, id, request).await?),
    ))
}

#[utoipa::path(
    post,
    path = "/api/items",
    tag = "stock",
    request_body = ItemCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Item created", body = ItemResponse),
        (status = 400, description = "Invalid item request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 409, description = "SKU already exists", body = String)
    )
)]
/// 创建库存物品。
pub(crate) async fn create_item(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<ItemCreateRequest>,
) -> Result<(StatusCode, Json<ItemResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item(&state, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/items",
    tag = "stock",
    params(ItemListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item list", body = PaginatedResponse<ItemResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 分页查询库存物品。
pub(crate) async fn list_items(
    State(state): State<CoreState>,
    Query(query): Query<ItemListQuery>,
) -> Result<Json<PaginatedResponse<ItemResponse>>, StockApiError> {
    Ok(Json(service::list_items(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item detail", body = ItemResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 查询单个库存物品。
pub(crate) async fn get_item(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<ItemResponse>, StockApiError> {
    Ok(Json(service::get_item(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/items/{id}",
    tag = "stock",
    request_body = ItemUpdateRequest,
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item updated", body = ItemResponse),
        (status = 400, description = "Invalid item request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 404, description = "Item not found", body = String),
        (status = 409, description = "SKU already exists", body = String)
    )
)]
/// 更新库存物品。
pub(crate) async fn update_item(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<ItemUpdateRequest>,
) -> Result<Json<ItemResponse>, StockApiError> {
    Ok(Json(service::update_item(&state, id, request).await?))
}

#[utoipa::path(
    delete,
    path = "/api/items/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Item deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Item manage permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 软删除库存物品。
pub(crate) async fn delete_item(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_item(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/items/{id}/substitutes",
    tag = "stock",
    request_body = SubstituteBindRequest,
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitutes replaced", body = Vec<SubstituteDetailResponse>),
        (status = 400, description = "Invalid substitute request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Substitute manage permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 整体替换指定物品的替代料列表。
pub(crate) async fn bind_substitutes(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    ValidatedJson(request): ValidatedJson<SubstituteBindRequest>,
) -> Result<Json<Vec<SubstituteDetailResponse>>, StockApiError> {
    Ok(Json(
        service::bind_substitutes(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}/substitutes",
    tag = "stock",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Substitute list", body = Vec<SubstituteDetailResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 查询指定物品的替代料列表。
pub(crate) async fn list_substitutes(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<SubstituteDetailResponse>>, StockApiError> {
    Ok(Json(service::list_substitutes(&state, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/items/{id}/substitutes/{substitute_id}",
    tag = "stock",
    params(
        ("id" = i64, Path, description = "Item ID"),
        ("substitute_id" = i64, Path, description = "Substitute item ID")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Substitute relation deleted"),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Substitute manage permission required", body = String),
        (status = 404, description = "Item or substitute relation not found", body = String)
    )
)]
/// 解绑单个替代料关系。
pub(crate) async fn delete_substitute(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((id, substitute_id)): Path<(i64, i64)>,
) -> Result<StatusCode, StockApiError> {
    service::delete_substitute(&state, &current_user, id, substitute_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/inbound",
    tag = "stock",
    request_body = InboundCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Inbound order created", body = InboundResponse),
        (status = 400, description = "Invalid inbound request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Inbound create permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 创建 pending 入库单；创建阶段不写库存批次或流水。
pub(crate) async fn create_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<InboundCreateRequest>,
) -> Result<(StatusCode, Json<InboundResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_inbound(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/inbound",
    tag = "stock",
    params(InboundListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order list", body = PaginatedResponse<InboundResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 分页查询入库单。
pub(crate) async fn list_inbound(
    State(state): State<CoreState>,
    Query(query): Query<InboundListQuery>,
) -> Result<Json<PaginatedResponse<InboundResponse>>, StockApiError> {
    Ok(Json(service::list_inbound(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/inbound/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order detail", body = InboundResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Inbound order not found", body = String)
    )
)]
/// 查询入库单详情。
pub(crate) async fn get_inbound(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(service::get_inbound(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/inbound/{id}/approve",
    tag = "stock",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order approved", body = InboundResponse),
        (status = 400, description = "Invalid inbound attributes", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Inbound approve permission required", body = String),
        (status = 404, description = "Inbound order not found", body = String),
        (status = 409, description = "Inbound order is not pending", body = String)
    )
)]
/// 审批 pending 入库单；审批事务会写批次、库存流水和审计事件。
pub(crate) async fn approve_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(
        service::approve_inbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/inbound/{id}/reject",
    tag = "stock",
    params(("id" = i64, Path, description = "Inbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Inbound order rejected", body = InboundResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Inbound approve permission required", body = String),
        (status = 404, description = "Inbound order not found", body = String),
        (status = 409, description = "Inbound order is not pending", body = String)
    )
)]
/// 拒绝 pending 入库单；拒绝不改变库存。
pub(crate) async fn reject_inbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<Json<InboundResponse>, StockApiError> {
    Ok(Json(
        service::reject_inbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/outbound",
    tag = "stock",
    request_body = OutboundCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Outbound order created", body = OutboundResponse),
        (status = 400, description = "Invalid outbound request", body = String),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Outbound create permission required", body = String),
        (status = 404, description = "Item not found", body = String)
    )
)]
/// 创建 pending 出库单；创建阶段不扣减库存。
pub(crate) async fn create_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<OutboundCreateRequest>,
) -> Result<(StatusCode, Json<OutboundResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_outbound(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/outbound",
    tag = "stock",
    params(OutboundListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order list", body = PaginatedResponse<OutboundResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 分页查询出库单。
pub(crate) async fn list_outbound(
    State(state): State<CoreState>,
    Query(query): Query<OutboundListQuery>,
) -> Result<Json<PaginatedResponse<OutboundResponse>>, StockApiError> {
    Ok(Json(service::list_outbound(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/outbound/{id}",
    tag = "stock",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order detail", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String),
        (status = 404, description = "Outbound order not found", body = String)
    )
)]
/// 查询出库单详情。
pub(crate) async fn get_outbound(
    State(state): State<CoreState>,
    Path(id): Path<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(service::get_outbound(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/outbound/{id}/approve",
    tag = "stock",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order approved", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Outbound approve permission required", body = String),
        (status = 404, description = "Outbound order not found", body = String),
        (status = 409, description = "Order is not pending or stock is insufficient", body = String)
    )
)]
/// 审批 pending 出库单；审批事务会按指定批次或 FIFO 扣减库存。
pub(crate) async fn approve_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(
        service::approve_outbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/outbound/{id}/reject",
    tag = "stock",
    params(("id" = i64, Path, description = "Outbound order ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Outbound order rejected", body = OutboundResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Outbound approve permission required", body = String),
        (status = 404, description = "Outbound order not found", body = String),
        (status = 409, description = "Outbound order is not pending", body = String)
    )
)]
/// 拒绝 pending 出库单；拒绝不扣减库存。
pub(crate) async fn reject_outbound(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<Json<OutboundResponse>, StockApiError> {
    Ok(Json(
        service::reject_outbound(&state, &current_user, id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/overview",
    tag = "stock",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Dashboard overview", body = DashboardOverviewResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 查询库存看板总览。
pub(crate) async fn dashboard_overview(
    State(state): State<CoreState>,
) -> Result<Json<DashboardOverviewResponse>, StockApiError> {
    Ok(Json(service::dashboard_overview(&state).await?))
}

#[utoipa::path(
    get,
    path = "/api/dashboard/trends",
    tag = "stock",
    params(TrendsQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Dashboard trends", body = TrendsResponse),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Stock read permission required", body = String)
    )
)]
/// 查询库存看板出入库趋势。
pub(crate) async fn dashboard_trends(
    State(state): State<CoreState>,
    Query(query): Query<TrendsQuery>,
) -> Result<Json<TrendsResponse>, StockApiError> {
    Ok(Json(service::dashboard_trends(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/events",
    tag = "stock",
    params(EventListQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Event log list", body = PaginatedResponse<EventLogResponse>),
        (status = 401, description = "Invalid access token", body = String),
        (status = 403, description = "Audit read permission required", body = String)
    )
)]
/// 分页查询事件日志。
pub(crate) async fn list_events(
    State(state): State<CoreState>,
    Query(query): Query<EventListQuery>,
) -> Result<Json<PaginatedResponse<EventLogResponse>>, StockApiError> {
    Ok(Json(service::list_events(&state, query).await?))
}

fn validate_positive_number(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
    }
}
