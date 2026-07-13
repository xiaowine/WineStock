//! 两类属性模板共用的字段校验与响应投影。
//!
//! 本模块属于 stock 服务层，只复用字段格式规则，不决定字段属于物品还是本次入库。

use std::collections::HashSet;

use super::super::{
    validation::{
        normalize_optional_text, normalize_required_text, parse_options_json, sqlite_bool,
        valid_iso_date, validate_http_url,
    },
    StockApiError,
};
use crate::{
    persistence::repository::{
        InboundTemplateDetail, ItemAttributeTemplateDetail, TemplateFieldInput,
    },
    stock::controller,
};

/// 归一化模板字段定义并生成稳定排序值。
pub(super) fn normalize_template_fields(
    fields: Vec<controller::TemplateFieldDef>,
) -> Result<Vec<TemplateFieldInput>, StockApiError> {
    if fields.is_empty() || fields.len() > 64 {
        return Err(StockApiError::InvalidRequest);
    }
    let mut names = HashSet::with_capacity(fields.len());
    let mut result = Vec::with_capacity(fields.len());
    for (index, field) in fields.into_iter().enumerate() {
        let name = normalize_required_text(&field.field_name)?;
        if !names.insert(name.to_lowercase()) {
            return Err(StockApiError::InvalidRequest);
        }
        let options = normalize_options(field.field_type, field.options)?;
        let default_value = normalize_optional_text(field.default_value)?;
        validate_default(
            field.field_type,
            default_value.as_deref(),
            options.as_deref(),
        )?;
        result.push(TemplateFieldInput {
            definition_id: None,
            field_name: name,
            field_type: field.field_type.as_code().to_owned(),
            required: field.required.unwrap_or(false),
            searchable: field.searchable.unwrap_or(false),
            options_json: options
                .map(|value| serde_json::to_string(&value))
                .transpose()
                .map_err(|_| StockApiError::InvalidRequest)?,
            default_value,
            unit_mode: "none".to_owned(),
            fixed_unit: None,
            unit_options_json: None,
            sort_order: index as i32,
        });
    }
    Ok(result)
}

/// 归一化物品属性模板字段，并验证各字段显式声明的单位规则。
pub(super) fn normalize_item_template_fields(
    fields: Vec<controller::ItemAttributeTemplateFieldDef>,
) -> Result<Vec<TemplateFieldInput>, StockApiError> {
    let parts = fields
        .into_iter()
        .map(controller::ItemAttributeTemplateFieldDef::into_parts)
        .collect::<Vec<_>>();
    let definition_ids = parts.iter().map(|part| part.0).collect::<Vec<_>>();
    let common_fields = parts.iter().map(|part| part.1.clone()).collect::<Vec<_>>();
    let unit_rules = parts.into_iter().map(|part| part.2).collect::<Vec<_>>();
    let mut normalized = normalize_template_fields(common_fields)?;
    for ((field, definition_id), rule) in normalized.iter_mut().zip(definition_ids).zip(unit_rules)
    {
        field.definition_id = definition_id;
        let (mode, fixed_unit, unit_options) = normalize_unit_rule(rule)?;
        field.unit_mode = mode.as_code().to_owned();
        field.fixed_unit = fixed_unit;
        field.unit_options_json = unit_options
            .map(|options| serde_json::to_string(&options))
            .transpose()
            .map_err(|_| StockApiError::InvalidRequest)?;
    }
    Ok(normalized)
}

fn normalize_unit_rule(
    rule: Option<controller::ItemAttributeUnitRule>,
) -> Result<
    (
        controller::ItemAttributeUnitMode,
        Option<String>,
        Option<Vec<String>>,
    ),
    StockApiError,
> {
    let Some(rule) = rule else {
        return Ok((controller::ItemAttributeUnitMode::None, None, None));
    };
    match rule.mode {
        controller::ItemAttributeUnitMode::None => {
            if rule.value.is_some() || rule.options.is_some() {
                return Err(StockApiError::InvalidRequest);
            }
            Ok((rule.mode, None, None))
        }
        controller::ItemAttributeUnitMode::Fixed => {
            if rule.options.is_some() {
                return Err(StockApiError::InvalidRequest);
            }
            let value = normalize_required_text(
                rule.value.as_deref().ok_or(StockApiError::InvalidRequest)?,
            )?;
            if value.chars().count() > 32 {
                return Err(StockApiError::InvalidRequest);
            }
            Ok((rule.mode, Some(value), None))
        }
        controller::ItemAttributeUnitMode::Select => {
            if rule.value.is_some() {
                return Err(StockApiError::InvalidRequest);
            }
            let options =
                normalize_unit_options(rule.options.ok_or(StockApiError::InvalidRequest)?)?;
            Ok((rule.mode, None, Some(options)))
        }
    }
}

fn normalize_unit_options(options: Vec<String>) -> Result<Vec<String>, StockApiError> {
    if options.is_empty() || options.len() > 32 {
        return Err(StockApiError::InvalidRequest);
    }
    let mut seen = HashSet::with_capacity(options.len());
    let mut result = Vec::with_capacity(options.len());
    for option in options {
        let option = normalize_required_text(&option)?;
        if option.chars().count() > 32 || !seen.insert(option.to_lowercase()) {
            return Err(StockApiError::InvalidRequest);
        }
        result.push(option);
    }
    Ok(result)
}

fn normalize_options(
    field_type: controller::TemplateFieldType,
    options: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, StockApiError> {
    match field_type {
        controller::TemplateFieldType::Select => {
            let Some(options) = options else {
                return Err(StockApiError::InvalidRequest);
            };
            if options.is_empty() || options.len() > 128 {
                return Err(StockApiError::InvalidRequest);
            }
            let mut seen = HashSet::with_capacity(options.len());
            let mut result = Vec::with_capacity(options.len());
            for option in options {
                let option = normalize_required_text(&option)?;
                if !seen.insert(option.to_lowercase()) {
                    return Err(StockApiError::InvalidRequest);
                }
                result.push(option);
            }
            Ok(Some(result))
        }
        _ if options.is_some() => Err(StockApiError::InvalidRequest),
        _ => Ok(None),
    }
}

fn validate_default(
    field_type: controller::TemplateFieldType,
    value: Option<&str>,
    options: Option<&[String]>,
) -> Result<(), StockApiError> {
    let Some(value) = value else {
        return Ok(());
    };
    match field_type {
        controller::TemplateFieldType::Number => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        controller::TemplateFieldType::Boolean if matches!(value, "true" | "false") => Ok(()),
        controller::TemplateFieldType::Boolean => Err(StockApiError::InvalidRequest),
        controller::TemplateFieldType::Url => validate_http_url(value),
        controller::TemplateFieldType::Date if valid_iso_date(value) => Ok(()),
        controller::TemplateFieldType::Date | controller::TemplateFieldType::File => {
            Err(StockApiError::InvalidRequest)
        }
        controller::TemplateFieldType::Select
            if options
                .unwrap_or_default()
                .iter()
                .any(|option| option == value) =>
        {
            Ok(())
        }
        controller::TemplateFieldType::Select => Err(StockApiError::InvalidRequest),
        _ => Ok(()),
    }
}

/// 把入库模板读取模型转换为 HTTP 响应。
pub(super) fn inbound_template_response(
    detail: InboundTemplateDetail,
) -> Result<controller::InboundTemplateResponse, StockApiError> {
    Ok(controller::InboundTemplateResponse {
        id: detail.template.id,
        name: detail.template.name,
        description: detail.template.description,
        fields: inbound_fields(detail.fields)?,
        created_at: detail.template.created_at,
        updated_at: detail.template.updated_at,
    })
}

/// 把物品属性模板读取模型转换为 HTTP 响应。
pub(super) fn item_attribute_template_response(
    detail: ItemAttributeTemplateDetail,
) -> Result<controller::ItemAttributeTemplateResponse, StockApiError> {
    Ok(controller::ItemAttributeTemplateResponse {
        id: detail.template.id,
        name: detail.template.name,
        description: detail.template.description,
        default_inbound_template_id: detail.template.default_inbound_template_id,
        fields: item_fields(detail.fields)?,
        created_at: detail.template.created_at,
        updated_at: detail.template.updated_at,
    })
}

fn inbound_fields(
    fields: Vec<crate::persistence::entity::inbound_template_field::Model>,
) -> Result<Vec<controller::TemplateFieldResponse>, StockApiError> {
    fields
        .into_iter()
        .map(|field| {
            field_response(
                field.id,
                field.field_name,
                field.field_type,
                field.required,
                field.searchable,
                field.options_json,
                field.default_value,
                field.sort_order,
            )
        })
        .collect()
}
fn item_fields(
    fields: Vec<crate::persistence::entity::item_attribute_definition::Model>,
) -> Result<Vec<controller::ItemAttributeTemplateFieldResponse>, StockApiError> {
    fields
        .into_iter()
        .map(|field| {
            let base = field_response(
                field.id,
                field.field_name,
                field.field_type,
                field.required,
                field.searchable,
                field.options_json,
                field.default_value,
                field.sort_order,
            )?;
            Ok(controller::ItemAttributeTemplateFieldResponse {
                field: base,
                unit: controller::ItemAttributeUnitRule {
                    mode: controller::ItemAttributeUnitMode::from_code(&field.unit_mode)?,
                    value: field.fixed_unit,
                    options: parse_options_json(field.unit_options_json)?,
                },
            })
        })
        .collect()
}

fn field_response(
    id: i64,
    field_name: String,
    field_type: String,
    required: i32,
    searchable: i32,
    options_json: Option<String>,
    default_value: Option<String>,
    sort_order: i32,
) -> Result<controller::TemplateFieldResponse, StockApiError> {
    Ok(controller::TemplateFieldResponse {
        id,
        field_name,
        field_type: controller::TemplateFieldType::from_code(&field_type)?,
        required: sqlite_bool(required),
        searchable: sqlite_bool(searchable),
        options: parse_options_json(options_json)?,
        default_value,
        sort_order,
    })
}
