//! 库存默认模板启动补齐。
//!
//! 本模块属于 `stock` 业务层，负责在本地服务启动时补齐内置库存模板。
//! 它不处理 HTTP 请求，也不覆盖或恢复用户已经创建或删除的同名模板。

use std::{error::Error, fmt};

use sea_orm::{DatabaseConnection, DbErr};

use crate::{
    persistence::repository::{CreateStockTemplate, StockRepository, TemplateFieldInput},
    stock::controller,
};

/// 库存默认模板启动补齐失败。
#[derive(Debug)]
pub enum StockBootstrapError {
    /// 读取或写入库存模板失败。
    Database(DbErr),
}

impl fmt::Display for StockBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(source) => write!(f, "failed to bootstrap stock defaults: {source}"),
        }
    }
}

impl Error for StockBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
        }
    }
}

impl From<DbErr> for StockBootstrapError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

/// 启动时补齐内置库存模板；同名记录存在时跳过，避免覆盖用户调整。
pub(crate) async fn bootstrap_default_templates(
    database: &DatabaseConnection,
) -> Result<(), StockBootstrapError> {
    let repository = StockRepository::new(database);
    for template in DEFAULT_TEMPLATES {
        if repository.template_name_exists(template.name).await? {
            continue;
        }
        repository
            .create_template(
                CreateStockTemplate {
                    name: template.name.to_owned(),
                    description: Some(template.description.to_owned()),
                    fields: template_field_inputs(template.fields),
                },
                None,
            )
            .await?;
    }

    Ok(())
}

struct BuiltinTemplateSpec {
    name: &'static str,
    description: &'static str,
    fields: &'static [BuiltinFieldSpec],
}

// 内置规格使用静态借用数据，启动时再转换为 repository 的写库输入模型。
struct BuiltinFieldSpec {
    field_name: &'static str,
    field_type: controller::TemplateFieldType,
    required: bool,
    searchable: bool,
    options_json: Option<&'static str>,
}

fn template_field_inputs(fields: &[BuiltinFieldSpec]) -> Vec<TemplateFieldInput> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| TemplateFieldInput {
            field_name: field.field_name.to_owned(),
            field_type: field.field_type.as_code().to_owned(),
            required: field.required,
            searchable: field.searchable,
            options_json: field.options_json.map(str::to_owned),
            default_value: None,
            sort_order: index as i32,
        })
        .collect()
}

const DEFAULT_TEMPLATES: &[BuiltinTemplateSpec] = &[
    BuiltinTemplateSpec {
        name: "元器件",
        description: "用于电子元器件入库，记录型号、封装、参数、包装方式和资料链接。",
        fields: &[
            BuiltinFieldSpec {
                field_name: "型号",
                field_type: controller::TemplateFieldType::Text,
                required: true,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "品牌",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "封装",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "参数",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "包装方式",
                field_type: controller::TemplateFieldType::Select,
                required: false,
                searchable: true,
                options_json: Some(r#"["散装","编带","托盘","管装","卷盘"]"#),
            },
            BuiltinFieldSpec {
                field_name: "数据手册",
                field_type: controller::TemplateFieldType::Url,
                required: false,
                searchable: false,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "购买链接",
                field_type: controller::TemplateFieldType::Url,
                required: false,
                searchable: false,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "备注",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: false,
                options_json: None,
            },
        ],
    },
    BuiltinTemplateSpec {
        name: "3D打印耗材",
        description: "用于 3D 打印耗材入库，记录材质、颜色、线径、重量和产品链接。",
        fields: &[
            BuiltinFieldSpec {
                field_name: "材质",
                field_type: controller::TemplateFieldType::Select,
                required: true,
                searchable: true,
                options_json: Some(r#"["PLA","PETG","ABS","TPU","ASA","PA","PC","树脂","其他"]"#),
            },
            BuiltinFieldSpec {
                field_name: "颜色",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "线径",
                field_type: controller::TemplateFieldType::Select,
                required: false,
                searchable: true,
                options_json: Some(r#"["1.75mm","2.85mm","其他"]"#),
            },
            BuiltinFieldSpec {
                field_name: "净重",
                field_type: controller::TemplateFieldType::Number,
                required: false,
                searchable: false,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "是否已开封",
                field_type: controller::TemplateFieldType::Boolean,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "干燥要求",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: false,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "产品链接",
                field_type: controller::TemplateFieldType::Url,
                required: false,
                searchable: false,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "备注",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: false,
                options_json: None,
            },
        ],
    },
    BuiltinTemplateSpec {
        name: "通用",
        description: "用于普通物品入库，记录品牌、规格型号、供应商、用途和备注。",
        fields: &[
            BuiltinFieldSpec {
                field_name: "品牌",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "规格型号",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "供应商",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "用途",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: true,
                options_json: None,
            },
            BuiltinFieldSpec {
                field_name: "备注",
                field_type: controller::TemplateFieldType::Text,
                required: false,
                searchable: false,
                options_json: None,
            },
        ],
    },
];
