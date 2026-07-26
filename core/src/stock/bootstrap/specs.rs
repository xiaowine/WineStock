//! 内置分类与模板的静态规格。
//!
//! 本模块属于 stock 业务层，只声明初始推荐数据，不执行数据库查询或覆盖用户数据。

use crate::{
    persistence::repository::{
        CreateItemAttributeTemplate, CreateItemCategory, TemplateFieldInput,
    },
    stock::controller::TemplateFieldType,
};

pub(super) struct CategorySpec {
    pub name: &'static str,
    pub description: &'static str,
    pub sort_order: i32,
}
pub(super) struct ItemTemplateSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub fields: &'static [FieldSpec],
}
pub(super) struct FieldSpec {
    pub name: &'static str,
    pub field_type: TemplateFieldType,
    pub required: bool,
    pub searchable: bool,
    pub options_json: Option<&'static str>,
}

pub(super) fn category_input(spec: &CategorySpec) -> CreateItemCategory {
    CreateItemCategory {
        name: spec.name.to_owned(),
        description: Some(spec.description.to_owned()),
        sort_order: spec.sort_order,
    }
}
pub(super) fn item_template_input(spec: &ItemTemplateSpec) -> CreateItemAttributeTemplate {
    CreateItemAttributeTemplate {
        name: spec.name.to_owned(),
        description: Some(spec.description.to_owned()),
        fields: fields(spec.fields),
    }
}
fn fields(specs: &[FieldSpec]) -> Vec<TemplateFieldInput> {
    specs
        .iter()
        .enumerate()
        .map(|(index, field)| TemplateFieldInput {
            definition_id: None,
            field_name: field.name.to_owned(),
            field_type: field.field_type.as_code().to_owned(),
            required: field.required,
            searchable: field.searchable,
            catalog_visible: false,
            options_json: field.options_json.map(str::to_owned),
            default_value: None,
            unit_mode: "none".to_owned(),
            fixed_unit: None,
            unit_options_json: None,
            sort_order: index as i32,
        })
        .collect()
}

pub(super) const DEFAULT_CATEGORIES: &[CategorySpec] = &[
    CategorySpec {
        name: "元器件",
        description: "电子元器件与模块",
        sort_order: 0,
    },
    CategorySpec {
        name: "3D打印耗材",
        description: "线材、树脂等打印耗材",
        sort_order: 10,
    },
    CategorySpec {
        name: "通用",
        description: "暂不适合其它分类的普通物品",
        sort_order: 100,
    },
];

pub(super) const DEFAULT_ITEM_TEMPLATES: &[ItemTemplateSpec] = &[
    ItemTemplateSpec {
        name: "元器件属性",
        description: "常见元器件固有参数预设，可继续添加任意自定义参数",
        fields: &[
            FieldSpec {
                name: "型号",
                field_type: TemplateFieldType::Text,
                required: true,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "品牌",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "封装",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "参数",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "数据手册",
                field_type: TemplateFieldType::Url,
                required: false,
                searchable: false,
                options_json: None,
            },
            FieldSpec {
                name: "产品图片",
                field_type: TemplateFieldType::File,
                required: false,
                searchable: false,
                options_json: None,
            },
        ],
    },
    ItemTemplateSpec {
        name: "3D打印耗材属性",
        description: "常见打印耗材固有参数预设",
        fields: &[
            FieldSpec {
                name: "材质",
                field_type: TemplateFieldType::Select,
                required: true,
                searchable: true,
                options_json: Some(r#"["PLA","PETG","ABS","TPU","ASA","PA","PC","树脂","其他"]"#),
            },
            FieldSpec {
                name: "颜色",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "线径",
                field_type: TemplateFieldType::Select,
                required: false,
                searchable: true,
                options_json: Some(r#"["1.75mm","2.85mm","其他"]"#),
            },
            FieldSpec {
                name: "品牌",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "产品链接",
                field_type: TemplateFieldType::Url,
                required: false,
                searchable: false,
                options_json: None,
            },
        ],
    },
    ItemTemplateSpec {
        name: "通用物品属性",
        description: "少量常用字段预设，不适用时可以完全不选择模板",
        fields: &[
            FieldSpec {
                name: "品牌",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "规格型号",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            FieldSpec {
                name: "用途",
                field_type: TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
        ],
    },
];
