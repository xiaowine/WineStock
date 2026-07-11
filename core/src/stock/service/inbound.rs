//! 入库单服务。
//!
//! 本模块属于 `stock` 业务服务层，负责入库单创建、分页、筛选值、详情、审批、拒绝和模板扩展属性校验。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::{
    files::stored_image_matches_metadata,
    persistence::repository::{
        CreateInboundOrder, CreateInboundOrderItem, FileObjectRepository, InboundAttributeInput,
        InboundOrderDetail, ListInboundOrders, StockRepository,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::{filter_values_response, inbound_response},
    validation::{
        normalize_optional_text, normalize_required_text, parse_attribute_object,
        parse_options_json, valid_iso_date, validate_http_url, validate_non_negative,
        validate_positive,
    },
    StockApiError,
};

/// 创建 pending 入库单；创建阶段只保存单据和明细，不改变库存数量。
///
/// 本函数会校验物品、库位、当前模板和图片所有权，并在同一事务绑定图片引用。
/// 库存批次与流水仍只在审批阶段写入。
pub(crate) async fn create_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    request: controller::InboundCreateRequest,
) -> Result<controller::InboundResponse, StockApiError> {
    if request.items.is_empty() || request.items.len() > 256 {
        return Err(StockApiError::InvalidRequest);
    }
    let repository = StockRepository::new(state.database());
    let mut items = Vec::with_capacity(request.items.len());
    let file_repository = FileObjectRepository::new(state.database());
    let mut used_file_ids = HashSet::new();
    for (line_index, item) in request.items.into_iter().enumerate() {
        let Some(stock_item) = repository.find_active_item_by_id(item.item_id).await? else {
            return Err(StockApiError::InboundItemInvalid {
                line_index,
                item_id: item.item_id,
            });
        };
        if repository
            .find_active_location_by_id(item.location_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::InboundLocationInvalid {
                line_index,
                location_id: item.location_id,
            });
        }
        let attributes = request_attribute_object(item.ext_attributes)?;
        let inbound_template_id = match item.inbound_template_id {
            Some(id) => Some(id),
            None => match stock_item.attribute_template_id {
                Some(id) => repository
                    .find_active_item_attribute_template_by_id(id)
                    .await?
                    .and_then(|detail| detail.template.default_inbound_template_id),
                None => None,
            },
        };
        let normalized_attributes = validate_create_attributes(
            &repository,
            &file_repository,
            current_user,
            state,
            line_index,
            inbound_template_id,
            &attributes,
        )
        .await?;
        for attribute in &normalized_attributes {
            let Some(file_id) = attribute.file_object_id else {
                continue;
            };
            if !used_file_ids.insert(file_id) {
                return Err(StockApiError::InboundFileUnavailable {
                    line_index,
                    field_name: attribute.field_name.clone(),
                    file_id,
                });
            }
        }
        items.push(CreateInboundOrderItem {
            item_id: item.item_id,
            quantity: validate_positive(item.quantity)?,
            unit_price: validate_non_negative(Some(item.unit_price))?.expect("输入值已存在"),
            location_id: item.location_id,
            batch_no: normalize_optional_text(item.batch_no)?,
            expires_at: normalize_optional_text(item.expires_at)?,
            inbound_template_id,
            attributes: normalized_attributes,
        });
    }

    let detail = repository
        .create_inbound_order(CreateInboundOrder {
            source: normalize_required_text(&request.source)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
            items,
        })
        .await
        .map_err(map_stock_db_error)?;

    inbound_response(detail)
}

/// 分页查询入库单；查询参数在这里统一归一化并转换为仓储查询输入。
pub(crate) async fn list_inbound(
    state: &CoreState,
    query: controller::InboundListQuery,
) -> Result<PaginatedResponse<controller::InboundResponse>, StockApiError> {
    if query.item_id.is_some_and(|id| id < 1) {
        return Err(StockApiError::InvalidRequest);
    }
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_inbound_orders(ListInboundOrders {
            page,
            page_size,
            item_id: query.item_id,
            date_from: normalize_optional_text(query.date_from)?,
            date_to: normalize_optional_text(query.date_to)?,
            search: normalize_optional_text(query.search)?,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result
            .items
            .into_iter()
            .map(inbound_response)
            .collect::<Result<Vec<_>, StockApiError>>()?,
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询入库历史视角下的筛选值；历史值不受当前库存余额影响。
pub(crate) async fn inbound_filter_values(
    state: &CoreState,
) -> Result<controller::FilterValuesResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    filter_values_response(repository.list_inbound_filter_values().await?)
}

/// 查询入库单详情；单据不存在时返回 `InboundOrderNotFound`。
pub(crate) async fn get_inbound(
    state: &CoreState,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_inbound_order_by_id(id).await? else {
        return Err(StockApiError::InboundOrderNotFound);
    };

    inbound_response(detail)
}

/// 审批入库单；审批前按物品关联模板校验扩展属性。
///
/// 通过校验后由 repository 在事务内写入批次、库存流水和审计事件。
pub(crate) async fn approve_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_inbound_order_by_id(id).await? else {
        return Err(StockApiError::InboundOrderNotFound);
    };
    if detail.order.status != "pending" {
        return Err(StockApiError::OrderNotPending);
    }
    validate_inbound_attributes(state, &repository, &detail).await?;

    let detail = repository
        .approve_inbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::InboundOrderNotFound)?;

    inbound_response(detail)
}

/// 拒绝入库单；拒绝不写库存批次或流水，只更新单据状态和审计信息。
pub(crate) async fn reject_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .reject_inbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::InboundOrderNotFound)?;

    inbound_response(detail)
}

/// 在入库审批边界再次校验当前模板和已绑定图片，防止模板或文件状态在等待期间失效。
async fn validate_inbound_attributes(
    state: &CoreState,
    repository: &StockRepository<'_>,
    detail: &InboundOrderDetail,
) -> Result<(), StockApiError> {
    let file_repository = FileObjectRepository::new(repository.database());
    for (line_index, item) in detail.items.iter().enumerate() {
        let Some(_stock_item) = repository.find_active_item_by_id(item.item_id).await? else {
            return Err(StockApiError::InboundItemInvalid {
                line_index,
                item_id: item.item_id,
            });
        };
        let attributes = parse_attribute_object(item.attributes_json.as_deref())?;
        let Some(template_id) = item.inbound_template_id else {
            if attributes.is_empty() {
                continue;
            }
            return Err(StockApiError::InboundFieldInvalid {
                line_index,
                field_name: "ext_attributes".to_owned(),
                reason: "template_missing",
            });
        };
        let Some(template) = repository
            .find_active_inbound_template_by_id(template_id)
            .await?
        else {
            return Err(StockApiError::InboundTemplateInvalid {
                line_index,
                template_id,
            });
        };

        let known_fields = template
            .fields
            .iter()
            .map(|field| field.field_name.as_str())
            .collect::<HashSet<_>>();
        if attributes
            .keys()
            .any(|name| !known_fields.contains(name.as_str()))
        {
            return Err(StockApiError::InboundFieldInvalid {
                line_index,
                field_name: "ext_attributes".to_owned(),
                reason: "unknown_field",
            });
        }
        for field in &template.fields {
            let value = attributes.get(&field.field_name);
            if field.required != 0 && value.is_none_or(is_empty_attribute_value) {
                return Err(invalid_field(line_index, &field.field_name, "required"));
            }
            let Some(value) = value else {
                continue;
            };
            validate_attribute_value(field, value)
                .map_err(|reason| invalid_field(line_index, &field.field_name, reason))?;
            if field.field_type == "file" {
                let file_id = file_id_from_value(value).ok_or_else(|| {
                    invalid_field(line_index, &field.field_name, "invalid_file_reference")
                })?;
                let record = file_repository
                    .find_access_record(file_id)
                    .await?
                    .ok_or_else(|| StockApiError::InboundFileUnavailable {
                        line_index,
                        field_name: field.field_name.clone(),
                        file_id,
                    })?;
                if record.inbound_order_item_id != Some(item.id)
                    || !record
                        .file
                        .mime_type
                        .as_deref()
                        .is_some_and(is_allowed_image_mime)
                    || !stored_image_matches_metadata(state.storage(), &record.file)
                {
                    return Err(StockApiError::InboundFileUnavailable {
                        line_index,
                        field_name: field.field_name.clone(),
                        file_id,
                    });
                }
            }
        }
    }

    Ok(())
}

/// 按模板字段类型校验单个扩展属性值。
fn validate_attribute_value(
    field: &crate::persistence::entity::inbound_template_field::Model,
    value: &Value,
) -> Result<(), &'static str> {
    if value.is_null() {
        return Ok(());
    }
    match controller::TemplateFieldType::from_code(&field.field_type)
        .map_err(|_| "invalid_field_type")?
    {
        controller::TemplateFieldType::Text => match value.as_str() {
            Some(text) if !text.trim().is_empty() => Ok(()),
            _ => Err("invalid_text"),
        },
        controller::TemplateFieldType::Date => match value.as_str() {
            Some(text) if valid_iso_date(text) => Ok(()),
            _ => Err("invalid_date"),
        },
        controller::TemplateFieldType::File => {
            if file_id_from_value(value).is_some() {
                Ok(())
            } else {
                Err("invalid_file_reference")
            }
        }
        controller::TemplateFieldType::Url => match value.as_str() {
            Some(text) => validate_http_url(text).map_err(|_| "invalid_url"),
            _ => Err("invalid_url"),
        },
        controller::TemplateFieldType::Number => match value.as_f64() {
            Some(number) if number.is_finite() => Ok(()),
            _ => Err("invalid_number"),
        },
        controller::TemplateFieldType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err("invalid_boolean")
            }
        }
        controller::TemplateFieldType::Select => {
            let Some(text) = value.as_str() else {
                return Err("invalid_select");
            };
            let options = parse_options_json(field.options_json.clone())
                .map_err(|_| "invalid_select")?
                .ok_or("invalid_select")?;
            if options.iter().any(|option| option == text) {
                Ok(())
            } else {
                Err("invalid_select")
            }
        }
    }
}

/// 判断模板 required 字段的属性值是否等价为空。
fn is_empty_attribute_value(value: &Value) -> bool {
    value.is_null() || matches!(value.as_str(), Some(text) if text.trim().is_empty())
}

/// 将创建请求中的扩展属性限制为一层 JSON 对象。
fn request_attribute_object(value: Option<Value>) -> Result<Map<String, Value>, StockApiError> {
    match value {
        None => Ok(Map::new()),
        Some(Value::Object(attributes)) => Ok(attributes),
        Some(_) => Err(StockApiError::InvalidRequest),
    }
}

/// 创建 pending 单据前按当前模板校验字段，并收集需要事务绑定的图片引用。
async fn validate_create_attributes(
    repository: &StockRepository<'_>,
    file_repository: &FileObjectRepository<'_>,
    current_user: &CurrentUser,
    state: &CoreState,
    line_index: usize,
    template_id: Option<i64>,
    attributes: &Map<String, Value>,
) -> Result<Vec<InboundAttributeInput>, StockApiError> {
    let Some(template_id) = template_id else {
        if attributes.is_empty() {
            return Ok(Vec::new());
        }
        return Err(invalid_field(
            line_index,
            "ext_attributes",
            "template_missing",
        ));
    };
    let Some(template) = repository
        .find_active_inbound_template_by_id(template_id)
        .await?
    else {
        return Err(StockApiError::InboundTemplateInvalid {
            line_index,
            template_id,
        });
    };
    let known_fields = template
        .fields
        .iter()
        .map(|field| field.field_name.as_str())
        .collect::<HashSet<_>>();
    if let Some(unknown) = attributes
        .keys()
        .find(|name| !known_fields.contains(name.as_str()))
    {
        return Err(invalid_field(line_index, unknown, "unknown_field"));
    }

    let mut normalized = Vec::new();
    for field in &template.fields {
        let value = attributes.get(&field.field_name);
        if field.required != 0 && value.is_none_or(is_empty_attribute_value) {
            return Err(invalid_field(line_index, &field.field_name, "required"));
        }
        let Some(value) = value else {
            continue;
        };
        validate_attribute_value(field, value)
            .map_err(|reason| invalid_field(line_index, &field.field_name, reason))?;
        let file_id = if field.field_type == "file" {
            let file_id = file_id_from_value(value).ok_or_else(|| {
                invalid_field(line_index, &field.field_name, "invalid_file_reference")
            })?;
            let record = file_repository
                .find_access_record(file_id)
                .await?
                .ok_or_else(|| file_unavailable(line_index, &field.field_name, file_id))?;
            if record.file.owner_user_id != Some(current_user.user_id)
                || record.is_bound()
                || !record
                    .file
                    .mime_type
                    .as_deref()
                    .is_some_and(is_allowed_image_mime)
                || !stored_image_matches_metadata(state.storage(), &record.file)
            {
                return Err(file_unavailable(line_index, &field.field_name, file_id));
            }
            Some(file_id)
        } else {
            None
        };
        normalized.push(InboundAttributeInput {
            template_field_id: Some(field.id),
            field_name: field.field_name.clone(),
            field_type: field.field_type.clone(),
            value_json: serde_json::to_string(value).map_err(|_| StockApiError::InvalidRequest)?,
            unit: None,
            sort_order: field.sort_order,
            file_object_id: file_id,
            file_owner_user_id: file_id.map(|_| current_user.user_id),
        });
    }
    Ok(normalized)
}

fn invalid_field(line_index: usize, field_name: &str, reason: &'static str) -> StockApiError {
    StockApiError::InboundFieldInvalid {
        line_index,
        field_name: field_name.to_owned(),
        reason,
    }
}

fn file_unavailable(line_index: usize, field_name: &str, file_id: i64) -> StockApiError {
    StockApiError::InboundFileUnavailable {
        line_index,
        field_name: field_name.to_owned(),
        file_id,
    }
}

fn file_id_from_value(value: &Value) -> Option<i64> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    let file_id = object.get("file_id")?.as_i64()?;
    (file_id > 0).then_some(file_id)
}

fn is_allowed_image_mime(value: &str) -> bool {
    matches!(value, "image/png" | "image/jpeg" | "image/webp")
}
