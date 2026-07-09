//! 库存控制器共用类型和校验函数。
//!
//! 本模块属于 `stock` HTTP 控制器层，只保存多个库存 HTTP 子模块共享的响应枚举和请求校验入口。
//! 它不访问数据库，也不承载具体业务流程。

use serde::{Deserialize, Serialize};

use crate::stock::service::StockApiError;

/// 筛选字段来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FilterFieldSource {
    /// 库存或入库列表内置字段。
    Base,

    /// 库存模板字段。
    Template,
}

impl FilterFieldSource {
    /// 从 repository 稳定代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "base" => Ok(Self::Base),
            "template" => Ok(Self::Template),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 筛选字段值类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FilterValueType {
    /// 普通文本。
    Text,

    /// 数值。
    Number,

    /// 固定选项。
    Select,

    /// 日期文本。
    Date,

    /// 文件引用。
    File,

    /// HTTP/HTTPS 链接。
    Url,

    /// 布尔值。
    Boolean,

    /// 同名模板字段跨模板类型不一致。
    Mixed,
}

impl FilterValueType {
    /// 从 repository 稳定代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "select" => Ok(Self::Select),
            "date" => Ok(Self::Date),
            "file" => Ok(Self::File),
            "url" => Ok(Self::Url),
            "boolean" => Ok(Self::Boolean),
            "mixed" => Ok(Self::Mixed),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 筛选值响应项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct FilterValueResponse {
    /// 筛选值，后端统一转为字符串。
    pub value: String,

    /// 命中数量；物品接口按物品去重，入库接口按入库单去重。
    pub count: u64,
}

/// 单个筛选字段及其可选值。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct FilterFieldResponse {
    /// 前端使用的稳定字段 key，例如 `base:unit` 或 `template:品牌`。
    pub key: String,

    /// 字段展示名称。
    pub label: String,

    /// 字段来源。
    pub source: FilterFieldSource,

    /// 字段值类型。
    pub value_type: FilterValueType,

    /// 当前视角下出现过的筛选值。
    pub values: Vec<FilterValueResponse>,
}

/// 筛选值接口响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct FilterValuesResponse {
    /// 可用于当前列表筛选的字段集合。
    pub fields: Vec<FilterFieldResponse>,
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

pub(super) fn validate_positive_number(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
    }
}
