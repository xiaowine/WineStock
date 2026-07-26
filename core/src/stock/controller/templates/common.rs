//! 两类属性模板共用的字段 HTTP DTO。
//!
//! 本模块属于 stock HTTP 层，只复用字段格式，不决定字段的业务归属。

use serde::{Deserialize, Serialize};

use crate::stock::service::StockApiError;
use crate::validation::{validate_not_blank, validate_optional_not_blank};

/// 模板字段类型。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TemplateFieldType {
    /// 普通文本字段。
    Text,
    /// 有限数值字段。
    Number,
    /// 预置选项字段。
    Select,
    /// 日期字符串字段。
    Date,
    /// 单张受控图片引用字段。
    File,
    /// HTTP 或 HTTPS URL 字段。
    Url,
    /// 布尔字段。
    Boolean,
}

impl TemplateFieldType {
    /// 返回数据库保存的稳定代码。
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Select => "select",
            Self::Date => "date",
            Self::File => "file",
            Self::Url => "url",
            Self::Boolean => "boolean",
        }
    }

    /// 从数据库稳定代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "select" => Ok(Self::Select),
            "date" => Ok(Self::Date),
            "file" => Ok(Self::File),
            "url" => Ok(Self::Url),
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
    /// 字段名称，同一模板内唯一。
    #[garde(length(min = 1, max = 64), custom(validate_not_blank))]
    pub field_name: String,
    /// 字段类型。
    #[garde(dive)]
    pub field_type: TemplateFieldType,
    /// 是否必填。
    #[garde(skip)]
    pub required: Option<bool>,
    /// 是否允许参与搜索。
    #[garde(skip)]
    pub searchable: Option<bool>,
    /// `select` 候选值。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,
    /// 可选默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
}

/// 模板字段响应。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
pub(crate) struct TemplateFieldResponse {
    /// 字段数据库 ID。
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
    /// 是否允许参与搜索。
    #[garde(skip)]
    pub searchable: bool,
    /// `select` 候选值。
    #[garde(inner(length(min = 1, max = 128)))]
    pub options: Option<Vec<String>>,
    /// 可选默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
    /// 字段展示顺序。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 复制任一属性模板时使用的新名称请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCopyRequest {
    /// 新模板名称。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
}
