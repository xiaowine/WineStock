//! 物品固有属性 HTTP DTO。
//!
//! 本模块属于 stock HTTP 层，描述物品自身的任意类型属性；它不包含批次或本次收货状态。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::templates::TemplateFieldType;
use crate::validation::{validate_not_blank, validate_optional_not_blank};

/// 创建或整体替换物品属性时的字段值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemAttributeRequest {
    /// 已有统一定义 ID；新自定义属性为空。
    #[garde(skip)]
    pub definition_id: Option<i64>,
    /// 属性名称，同一物品内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 属性类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
    /// 自定义 select 候选项。
    #[garde(skip)]
    pub options: Option<Vec<String>>,
    /// number 单位模式。
    #[garde(skip)]
    pub unit_mode: Option<String>,
    /// fixed 模式的固定单位。
    #[garde(skip)]
    pub fixed_unit: Option<String>,
    /// select 模式的单位候选项。
    #[garde(skip)]
    pub unit_options: Option<Vec<String>>,
    /// 类型化 JSON 值；file 使用 `{ "file_id": id }`。
    #[garde(skip)]
    pub value: Value,
    /// 可选计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,
}

/// 物品固有属性响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemAttributeResponse {
    /// 属性数据库 ID。
    #[garde(skip)]
    pub id: i64,
    /// 统一定义 ID。
    #[garde(skip)]
    pub definition_id: i64,
    /// 是否为物品私有自定义定义。
    #[garde(skip)]
    pub custom: bool,
    /// 属性名称。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 属性类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
    /// select 候选项。
    #[garde(skip)]
    pub options: Option<Vec<String>>,
    /// 单位模式。
    #[garde(skip)]
    pub unit_mode: String,
    /// 固定单位。
    #[garde(skip)]
    pub fixed_unit: Option<String>,
    /// 单位候选项。
    #[garde(skip)]
    pub unit_options: Option<Vec<String>>,
    /// 类型化 JSON 值。
    #[garde(skip)]
    pub value: Value,
    /// 可选计量单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,
    /// 属性展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}
