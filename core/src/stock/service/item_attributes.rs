//! 物品固有属性的归一化、模板预设校验和响应投影。
//!
//! 本模块属于 stock 服务层，允许模板字段与任意自定义字段共存；它不处理入库批次属性。

use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::{
    validation::{
        normalize_optional_text, normalize_required_text, parse_options_json, valid_iso_date,
        validate_http_url,
    },
    StockApiError,
};
use crate::{
    persistence::repository::{
        FileObjectRepository, ItemAttributeInput, ItemAttributeRecord, StockRepository,
    },
    security::CurrentUser,
    stock::controller,
};

/// 归一化物品属性，校验可选模板字段以及新上传或当前物品已有 file 引用。
pub(super) async fn normalize_item_attributes(
    repository: &StockRepository<'_>,
    user: &CurrentUser,
    template_id: Option<i64>,
    current_item_id: Option<i64>,
    requests: Vec<controller::ItemAttributeRequest>,
) -> Result<Vec<ItemAttributeInput>, StockApiError> {
    if requests.len() > 128 {
        return Err(StockApiError::InvalidRequest);
    }
    let template = match template_id {
        Some(id) => Some(
            repository
                .find_active_item_attribute_template_by_id(id)
                .await?
                .ok_or(StockApiError::TemplateNotFound)?,
        ),
        None => None,
    };
    let template_fields = template
        .as_ref()
        .map(|detail| {
            detail
                .fields
                .iter()
                .map(|field| (field.field_name.to_lowercase(), field))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut seen = HashSet::with_capacity(requests.len());
    let mut used_file_ids = HashSet::new();
    let file_repository = FileObjectRepository::new(repository.database());
    let mut result = Vec::with_capacity(requests.len());
    for (index, request) in requests.into_iter().enumerate() {
        let field_name = normalize_required_text(&request.field_name)?;
        if !seen.insert(field_name.to_lowercase()) {
            return Err(StockApiError::InvalidRequest);
        }
        let preset = template_fields.get(&field_name.to_lowercase()).copied();
        if request
            .template_field_id
            .is_some_and(|id| preset.is_none_or(|field| field.id != id))
        {
            return Err(StockApiError::InvalidRequest);
        }
        if preset.is_some_and(|field| field.field_type != request.field_type.as_code()) {
            return Err(StockApiError::InvalidRequest);
        }
        validate_value(
            request.field_type,
            &request.value,
            preset.and_then(|field| field.options_json.clone()),
        )?;
        let file_id = if request.field_type == controller::TemplateFieldType::File {
            let id = file_id(&request.value).ok_or(StockApiError::InvalidRequest)?;
            let record = file_repository
                .find_access_record(id)
                .await?
                .ok_or(StockApiError::InvalidRequest)?;
            let already_bound_to_current_item = current_item_id.is_some_and(|item_id| {
                record.item_id == Some(item_id) && record.inbound_order_item_id.is_none()
            });
            let owned_unbound =
                record.file.owner_user_id == Some(user.user_id) && !record.is_bound();
            if !already_bound_to_current_item && !owned_unbound {
                return Err(StockApiError::InvalidRequest);
            }
            if !used_file_ids.insert(id) {
                return Err(StockApiError::InvalidRequest);
            }
            Some(id)
        } else {
            None
        };
        result.push(ItemAttributeInput {
            template_field_id: preset.map(|field| field.id),
            field_name,
            field_type: request.field_type.as_code().to_owned(),
            value_json: serde_json::to_string(&request.value)
                .map_err(|_| StockApiError::InvalidRequest)?,
            unit: normalize_optional_text(request.unit)?,
            sort_order: index as i32,
            file_object_id: file_id,
            file_owner_user_id: file_id.map(|_| user.user_id),
        });
    }
    if let Some(template) = template {
        for field in template.fields {
            if field.required != 0 && !seen.contains(&field.field_name.to_lowercase()) {
                return Err(StockApiError::InvalidRequest);
            }
        }
    }
    Ok(result)
}

/// 把物品属性数据库记录恢复为类型化 HTTP 值。
pub(super) fn item_attribute_responses(
    attributes: Vec<ItemAttributeRecord>,
) -> Result<Vec<controller::ItemAttributeResponse>, StockApiError> {
    attributes
        .into_iter()
        .map(|attribute| {
            Ok(controller::ItemAttributeResponse {
                id: attribute.id,
                template_field_id: attribute.template_field_id,
                field_name: attribute.field_name,
                field_type: controller::TemplateFieldType::from_code(&attribute.field_type)?,
                value: serde_json::from_str(&attribute.value_json)
                    .map_err(|_| StockApiError::InvalidRequest)?,
                unit: attribute.unit,
                sort_order: attribute.sort_order,
            })
        })
        .collect()
}

fn validate_value(
    field_type: controller::TemplateFieldType,
    value: &Value,
    options_json: Option<String>,
) -> Result<(), StockApiError> {
    match field_type {
        controller::TemplateFieldType::Text => value
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        controller::TemplateFieldType::Number => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        controller::TemplateFieldType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        controller::TemplateFieldType::Url => value
            .as_str()
            .ok_or(StockApiError::InvalidRequest)
            .and_then(validate_http_url),
        controller::TemplateFieldType::Date => value
            .as_str()
            .filter(|text| valid_iso_date(text))
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        controller::TemplateFieldType::File => {
            if file_id(value).is_some() {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        controller::TemplateFieldType::Select => {
            let text = value.as_str().ok_or(StockApiError::InvalidRequest)?;
            match parse_options_json(options_json)? {
                Some(options) if options.iter().any(|option| option == text) => Ok(()),
                Some(_) => Err(StockApiError::InvalidRequest),
                None if !text.trim().is_empty() => Ok(()),
                None => Err(StockApiError::InvalidRequest),
            }
        }
    }
}

fn file_id(value: &Value) -> Option<i64> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object.get("file_id")?.as_i64().filter(|id| *id > 0)
}
