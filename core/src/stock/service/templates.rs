//! 库存模板服务。
//!
//! 本模块属于 `stock` 业务服务层，负责模板 CRUD、复制、字段组合校验和模板响应组装调用。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use std::collections::HashSet;

use crate::{
    persistence::repository::{
        CreateStockTemplate, StockRepository, TemplateFieldInput, UpdateStockTemplate,
    },
    state::CoreState,
    stock::controller,
};

use super::{
    response::template_response,
    validation::{normalize_optional_text, normalize_required_text},
    StockApiError,
};

/// 创建库存模板；会写入模板主表和字段定义。
///
/// 名称冲突返回 `TemplateNameTaken`，字段组合不合法返回 `InvalidRequest`。
pub(crate) async fn create_template(
    state: &CoreState,
    request: controller::TemplateCreateRequest,
) -> Result<controller::TemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }
    let detail = repository
        .create_template(CreateStockTemplate {
            name,
            description: normalize_optional_text(request.description)?,
            fields: normalize_template_fields(request.fields)?,
        })
        .await?;

    template_response(detail)
}

/// 查询库存模板列表；只返回未软删除模板和对应字段定义。
pub(crate) async fn list_templates(
    state: &CoreState,
) -> Result<Vec<controller::TemplateResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    repository
        .list_active_templates()
        .await?
        .into_iter()
        .map(template_response)
        .collect()
}

/// 查询单个库存模板；模板不存在或已软删除时返回 `TemplateNotFound`。
pub(crate) async fn get_template(
    state: &CoreState,
    id: i64,
) -> Result<controller::TemplateResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_active_template_by_id(id).await? else {
        return Err(StockApiError::TemplateNotFound);
    };

    template_response(detail)
}

/// 更新库存模板；字段存在时会整体替换旧字段定义。
///
/// 本函数会检查模板名称唯一性，并保持“未传字段不修改”的 HTTP 语义。
pub(crate) async fn update_template(
    state: &CoreState,
    id: i64,
    request: controller::TemplateUpdateRequest,
) -> Result<controller::TemplateResponse, StockApiError> {
    let name = request
        .name
        .map(|name| normalize_required_text(&name))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(name) = name.as_deref() {
        if repository
            .active_template_name_exists_except(name, Some(id))
            .await?
        {
            return Err(StockApiError::TemplateNameTaken);
        }
    }

    let fields = request.fields.map(normalize_template_fields).transpose()?;
    let Some(detail) = repository
        .update_template(
            id,
            UpdateStockTemplate {
                name,
                description: request
                    .description
                    .map(|description| normalize_required_text(&description))
                    .transpose()?
                    .map(Some),
                fields,
            },
        )
        .await?
    else {
        return Err(StockApiError::TemplateNotFound);
    };

    template_response(detail)
}

/// 软删除库存模板；仍有关联未软删除物品时拒绝删除。
pub(crate) async fn delete_template(state: &CoreState, id: i64) -> Result<(), StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.active_items_reference_template(id).await? {
        return Err(StockApiError::TemplateInUse);
    }
    if repository.soft_delete_template(id).await? {
        Ok(())
    } else {
        Err(StockApiError::TemplateNotFound)
    }
}

/// 复制库存模板及字段定义；源模板不存在时返回 `TemplateNotFound`。
pub(crate) async fn copy_template(
    state: &CoreState,
    id: i64,
    request: controller::TemplateCopyRequest,
) -> Result<controller::TemplateResponse, StockApiError> {
    let name = normalize_required_text(&request.name)?;
    let repository = StockRepository::new(state.database());
    if repository
        .active_template_name_exists_except(&name, None)
        .await?
    {
        return Err(StockApiError::TemplateNameTaken);
    }

    let Some(detail) = repository.copy_template(id, name).await? else {
        return Err(StockApiError::TemplateNotFound);
    };

    template_response(detail)
}

/// 归一化模板字段定义；会校验数量、名称唯一性、字段选项和默认值。
fn normalize_template_fields(
    fields: Vec<controller::TemplateFieldDef>,
) -> Result<Vec<TemplateFieldInput>, StockApiError> {
    if fields.is_empty() || fields.len() > 64 {
        return Err(StockApiError::InvalidRequest);
    }
    let mut names = HashSet::with_capacity(fields.len());
    let mut normalized = Vec::with_capacity(fields.len());

    for (index, field) in fields.into_iter().enumerate() {
        let field_name = normalize_required_text(&field.field_name)?;
        if !names.insert(field_name.to_lowercase()) {
            return Err(StockApiError::InvalidRequest);
        }
        let options = normalize_field_options(field.field_type, field.options)?;
        let default_value = normalize_optional_text(field.default_value)?;
        validate_field_default(
            field.field_type,
            default_value.as_deref(),
            options.as_deref(),
        )?;
        normalized.push(TemplateFieldInput {
            field_name,
            field_type: field.field_type.as_code().to_owned(),
            required: field.required.unwrap_or(false),
            searchable: field.searchable.unwrap_or(false),
            options_json: options
                .map(|options| serde_json::to_string(&options))
                .transpose()
                .map_err(|_| StockApiError::InvalidRequest)?,
            default_value,
            sort_order: index as i32,
        });
    }

    Ok(normalized)
}

/// 归一化 select 字段选项；非 select 字段传入 options 会被视为请求错误。
fn normalize_field_options(
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
            let mut normalized = Vec::with_capacity(options.len());
            for option in options {
                let option = normalize_required_text(&option)?;
                if !seen.insert(option.to_lowercase()) {
                    return Err(StockApiError::InvalidRequest);
                }
                normalized.push(option);
            }
            Ok(Some(normalized))
        }
        _ if options.is_some() => Err(StockApiError::InvalidRequest),
        _ => Ok(None),
    }
}

/// 校验模板默认值是否能被对应字段类型解释。
fn validate_field_default(
    field_type: controller::TemplateFieldType,
    default_value: Option<&str>,
    options: Option<&[String]>,
) -> Result<(), StockApiError> {
    let Some(default_value) = default_value else {
        return Ok(());
    };

    match field_type {
        controller::TemplateFieldType::Number => default_value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        controller::TemplateFieldType::Boolean => {
            if matches!(default_value, "true" | "false") {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        controller::TemplateFieldType::Select => {
            if options
                .unwrap_or_default()
                .iter()
                .any(|option| option == default_value)
            {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        _ => Ok(()),
    }
}
