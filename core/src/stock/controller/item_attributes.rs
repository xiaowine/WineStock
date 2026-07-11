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
    /// 可选模板字段来源；自定义属性为空。
    #[garde(skip)]
    pub template_field_id: Option<i64>,
    /// 属性名称，同一物品内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 属性类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
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
    /// 可选模板字段来源。
    #[garde(skip)]
    pub template_field_id: Option<i64>,
    /// 属性名称。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 属性类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
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
