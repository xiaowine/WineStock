//! 入库单服务。
//!
//! 本模块属于 `stock` 业务服务层，负责入库单创建、分页、详情、审批、拒绝和模板扩展属性校验。
//! 它不处理 HTTP 路由、权限中间件或数据库表细节。

use serde_json::Value;
use std::collections::HashSet;

use crate::{
    persistence::repository::{
        CreateInboundOrder, CreateInboundOrderItem, InboundOrderDetail, ListInboundOrders,
        StockRepository,
    },
    security::CurrentUser,
    state::CoreState,
    stock::controller,
};

use super::{
    error::map_stock_db_error,
    pagination::{total_pages, PaginatedResponse, DEFAULT_PAGE, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    response::inbound_response,
    validation::{
        normalize_optional_text, normalize_required_text, parse_attribute_object,
        parse_options_json, validate_non_negative, validate_positive,
    },
    StockApiError,
};

/// 创建 pending 入库单；创建阶段只保存单据和明细，不改变库存数量。
///
/// 本函数会校验明细物品存在性和数值边界，库存批次与流水只在审批阶段写入。
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
    for item in request.items {
        if repository
            .find_active_item_by_id(item.item_id)
            .await?
            .is_none()
        {
            return Err(StockApiError::ItemNotFound);
        }
        items.push(CreateInboundOrderItem {
            item_id: item.item_id,
            quantity: validate_positive(item.quantity)?,
            unit_price: validate_non_negative(Some(item.unit_price))?.expect("输入值已存在"),
            location: normalize_optional_text(item.location)?,
            batch_no: normalize_optional_text(item.batch_no)?,
            expires_at: normalize_optional_text(item.expires_at)?,
            ext_attributes_json: item
                .ext_attributes
                .map(|value| serde_json::to_string(&value))
                .transpose()
                .map_err(|_| StockApiError::InvalidRequest)?,
        });
    }

    let detail = repository
        .create_inbound_order(CreateInboundOrder {
            source: normalize_required_text(&request.source)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
            items,
        })
        .await?;

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
    validate_inbound_attributes(&repository, &detail).await?;

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

/// 在入库审批边界校验模板扩展属性；创建 pending 单据时只保存原始输入。
async fn validate_inbound_attributes(
    repository: &StockRepository<'_>,
    detail: &InboundOrderDetail,
) -> Result<(), StockApiError> {
    for item in &detail.items {
        let Some(stock_item) = repository.find_active_item_by_id(item.item_id).await? else {
            return Err(StockApiError::ItemNotFound);
        };
        let attributes = parse_attribute_object(item.ext_attributes_json.as_deref())?;
        let Some(template_id) = stock_item.category_id else {
            if attributes.is_empty() {
                continue;
            }
            return Err(StockApiError::InvalidRequest);
        };
        let Some(template) = repository.find_active_template_by_id(template_id).await? else {
            return Err(StockApiError::TemplateNotFound);
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
            return Err(StockApiError::InvalidRequest);
        }
        for field in &template.fields {
            let value = attributes.get(&field.field_name);
            if field.required != 0 && value.is_none_or(is_empty_attribute_value) {
                return Err(StockApiError::InvalidRequest);
            }
            let Some(value) = value else {
                continue;
            };
            validate_attribute_value(field, value)?;
        }
    }

    Ok(())
}

/// 按模板字段类型校验单个扩展属性值。
fn validate_attribute_value(
    field: &crate::persistence::entity::stock_template_field::Model,
    value: &Value,
) -> Result<(), StockApiError> {
    if value.is_null() {
        return Ok(());
    }
    match controller::TemplateFieldType::from_code(&field.field_type)? {
        controller::TemplateFieldType::Text
        | controller::TemplateFieldType::Date
        | controller::TemplateFieldType::File => match value.as_str() {
            Some(text) if !text.trim().is_empty() => Ok(()),
            _ => Err(StockApiError::InvalidRequest),
        },
        controller::TemplateFieldType::Number => match value.as_f64() {
            Some(number) if number.is_finite() => Ok(()),
            _ => Err(StockApiError::InvalidRequest),
        },
        controller::TemplateFieldType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        controller::TemplateFieldType::Select => {
            let Some(text) = value.as_str() else {
                return Err(StockApiError::InvalidRequest);
            };
            let options = parse_options_json(field.options_json.clone())?
                .ok_or(StockApiError::InvalidRequest)?;
            if options.iter().any(|option| option == text) {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
    }
}

/// 判断模板 required 字段的属性值是否等价为空。
fn is_empty_attribute_value(value: &Value) -> bool {
    value.is_null() || matches!(value.as_str(), Some(text) if text.trim().is_empty())
}
