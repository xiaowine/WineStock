//! 物品属性模板和入库模板仓储模型。
//!
//! 本模块属于 `core` 持久化层；两类模板独立，只共享字段定义结构。

use crate::{
    persistence::entity::{
        inbound_template, inbound_template_field, item_attribute_definition,
        item_attribute_template,
    },
    validation::{validate_not_blank, validate_optional_json_text, validate_optional_not_blank},
};

/// 创建模板字段定义的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct TemplateFieldInput {
    /// 已有物品属性定义 ID；新字段和入库模板字段为空。
    #[garde(skip)]
    pub definition_id: Option<i64>,
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
    #[garde(
        length(min = 1, max = 4096),
        custom(validate_optional_not_blank),
        custom(validate_optional_json_text)
    )]
    pub options_json: Option<String>,
    /// 默认值。
    #[garde(length(min = 1, max = 256), custom(validate_optional_not_blank))]
    pub default_value: Option<String>,
    /// 物品模板字段单位规则；入库模板固定使用 none。
    #[garde(length(min = 1, max = 16), custom(validate_not_blank))]
    pub unit_mode: String,
    /// fixed 模式的固定单位。
    #[garde(length(min = 1, max = 32), custom(validate_optional_not_blank))]
    pub fixed_unit: Option<String>,
    /// select 模式的单位候选值 JSON。
    #[garde(
        length(min = 1, max = 2048),
        custom(validate_optional_not_blank),
        custom(validate_optional_json_text)
    )]
    pub unit_options_json: Option<String>,
    /// 字段排序，从 0 开始。
    #[garde(range(min = 0))]
    pub sort_order: i32,
}

/// 创建入库属性模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateInboundTemplate {
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

/// 更新入库属性模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateInboundTemplate {
    /// 模板名称，存在时未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 模板说明；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,
    /// 模板字段定义；存在时整体替换旧字段。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldInput>>,
}

/// 入库属性模板详情。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundTemplateDetail {
    /// 入库模板基础资料。
    pub template: inbound_template::Model,
    /// 入库模板字段定义，按展示顺序返回。
    pub fields: Vec<inbound_template_field::Model>,
}

/// 创建物品属性模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct CreateItemAttributeTemplate {
    /// 模板名称，未软删除记录内唯一。
    #[garde(length(min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,
    /// 模板说明。
    #[garde(length(min = 1, max = 1024), custom(validate_optional_not_blank))]
    pub description: Option<String>,
    /// 推荐的默认入库模板 ID。
    #[garde(skip)]
    pub default_inbound_template_id: Option<i64>,
    /// 预设字段定义。
    #[garde(dive)]
    pub fields: Vec<TemplateFieldInput>,
}

/// 更新物品属性模板的仓储输入。
#[derive(Debug, Clone, PartialEq, Eq, garde::Validate)]
pub(crate) struct UpdateItemAttributeTemplate {
    /// 模板名称；为空表示不修改。
    #[garde(length(min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,
    /// 模板说明；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub description: Option<Option<String>>,
    /// 默认入库模板；外层表示是否修改，内层表示是否清空。
    #[garde(skip)]
    pub default_inbound_template_id: Option<Option<i64>>,
    /// 预设字段；存在时整体替换。
    #[garde(skip)]
    pub fields: Option<Vec<TemplateFieldInput>>,
}

/// 物品属性模板详情。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemAttributeTemplateDetail {
    /// 物品属性模板基础资料。
    pub template: item_attribute_template::Model,
    /// 预设字段定义，按展示顺序返回。
    pub fields: Vec<item_attribute_definition::Model>,
}
