//! 库存服务响应组装。
//!
//! 本模块属于 `stock` 业务服务层，负责把 repository 记录投影为库存 HTTP DTO。
//! 它不执行数据库查询，也不处理授权或路由。

use serde_json::Value;

use crate::{
    persistence::{
        entity::stock_item,
        repository::{
            AuditEventRecord, DashboardOverviewRecord, InboundOrderDetail, OutboundOrderDetail,
            StockFilterFieldRecord, StockItemDetail, StockSubstituteRecord, StockTemplateDetail,
        },
    },
    stock::controller,
};

use super::{
    validation::{parse_options_json, sqlite_bool},
    StockApiError,
};

/// 把库存物品数据库模型转换为 HTTP 响应，不暴露软删除字段。
pub(super) fn item_response(item: stock_item::Model) -> controller::ItemResponse {
    controller::ItemResponse {
        id: item.id,
        name: item.name,
        sku: item.sku,
        category_id: item.category_id,
        unit: item.unit,
        description: item.description,
        default_price: item.default_price,
        reorder_point: item.reorder_point,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

/// 把库存物品详情读取模型转换为 HTTP 响应，库存聚合只反映当前有效批次。
pub(super) fn item_detail_response(detail: StockItemDetail) -> controller::ItemDetailResponse {
    let item = item_response(detail.item);

    controller::ItemDetailResponse {
        id: item.id,
        name: item.name,
        sku: item.sku,
        category_id: item.category_id,
        unit: item.unit,
        description: item.description,
        default_price: item.default_price,
        reorder_point: item.reorder_point,
        created_at: item.created_at,
        updated_at: item.updated_at,
        current_quantity: detail.current_quantity,
        inventory_value: detail.inventory_value,
        locations: detail
            .locations
            .into_iter()
            .map(|location| controller::ItemLocationStockResponse {
                location: location.location,
                quantity: location.quantity,
                value: location.value,
                batch_count: location.batch_count,
            })
            .collect(),
        batches: detail
            .batches
            .into_iter()
            .map(|batch| controller::ItemBatchStockResponse {
                id: batch.id,
                batch_no: batch.batch_no,
                location: batch.location,
                initial_quantity: batch.initial_quantity,
                remaining_quantity: batch.remaining_quantity,
                unit_cost: batch.unit_cost,
                value: batch.value,
                received_at: batch.received_at,
                expires_at: batch.expires_at,
            })
            .collect(),
    }
}

/// 把筛选值聚合记录转换为 HTTP 响应；字段来源和类型代码必须是服务端已知值。
pub(super) fn filter_values_response(
    fields: Vec<StockFilterFieldRecord>,
) -> Result<controller::FilterValuesResponse, StockApiError> {
    Ok(controller::FilterValuesResponse {
        fields: fields
            .into_iter()
            .map(|field| {
                Ok(controller::FilterFieldResponse {
                    key: field.key,
                    label: field.label,
                    source: controller::FilterFieldSource::from_code(&field.source)?,
                    value_type: controller::FilterValueType::from_code(&field.value_type)?,
                    values: field
                        .values
                        .into_iter()
                        .map(|value| controller::FilterValueResponse {
                            value: value.value,
                            count: value.count,
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, StockApiError>>()?,
    })
}

/// 把模板详情记录转换为 HTTP 响应；会把字段类型代码和 options JSON 恢复为 API 结构。
pub(super) fn template_response(
    detail: StockTemplateDetail,
) -> Result<controller::TemplateResponse, StockApiError> {
    let fields = detail
        .fields
        .into_iter()
        .map(|field| {
            Ok(controller::TemplateFieldResponse {
                id: field.id,
                field_name: field.field_name,
                field_type: controller::TemplateFieldType::from_code(&field.field_type)?,
                required: sqlite_bool(field.required),
                searchable: sqlite_bool(field.searchable),
                options: parse_options_json(field.options_json)?,
                default_value: field.default_value,
                sort_order: field.sort_order,
            })
        })
        .collect::<Result<Vec<_>, StockApiError>>()?;

    Ok(controller::TemplateResponse {
        id: detail.template.id,
        name: detail.template.name,
        description: detail.template.description,
        fields,
        created_at: detail.template.created_at,
        updated_at: detail.template.updated_at,
    })
}

/// 把入库单详情转换为 HTTP 响应；扩展属性 JSON 解析失败时返回 `InvalidRequest`。
pub(super) fn inbound_response(
    detail: InboundOrderDetail,
) -> Result<controller::InboundResponse, StockApiError> {
    Ok(controller::InboundResponse {
        id: detail.order.id,
        source: detail.order.source,
        status: controller::OrderStatus::from_code(&detail.order.status)?,
        notes: detail.order.notes,
        created_by_user_id: detail.order.created_by_user_id,
        approved_by_user_id: detail.order.approved_by_user_id,
        rejected_by_user_id: detail.order.rejected_by_user_id,
        created_at: detail.order.created_at,
        updated_at: detail.order.updated_at,
        approved_at: detail.order.approved_at,
        rejected_at: detail.order.rejected_at,
        items: detail
            .items
            .into_iter()
            .map(|item| {
                Ok(controller::InboundItemResponse {
                    id: item.id,
                    order_id: item.order_id,
                    item_id: item.item_id,
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    location: item.location,
                    batch_no: item.batch_no,
                    expires_at: item.expires_at,
                    ext_attributes: item
                        .ext_attributes_json
                        .map(|json| serde_json::from_str(&json))
                        .transpose()
                        .map_err(|_| StockApiError::InvalidRequest)?,
                    created_at: item.created_at,
                })
            })
            .collect::<Result<Vec<_>, StockApiError>>()?,
    })
}

/// 把出库单详情转换为 HTTP 响应；只投影已持久化的单据和明细。
pub(super) fn outbound_response(
    detail: OutboundOrderDetail,
) -> Result<controller::OutboundResponse, StockApiError> {
    Ok(controller::OutboundResponse {
        id: detail.order.id,
        destination: detail.order.destination,
        status: controller::OrderStatus::from_code(&detail.order.status)?,
        notes: detail.order.notes,
        created_by_user_id: detail.order.created_by_user_id,
        approved_by_user_id: detail.order.approved_by_user_id,
        rejected_by_user_id: detail.order.rejected_by_user_id,
        created_at: detail.order.created_at,
        updated_at: detail.order.updated_at,
        approved_at: detail.order.approved_at,
        rejected_at: detail.order.rejected_at,
        items: detail
            .items
            .into_iter()
            .map(|item| controller::OutboundItemResponse {
                id: item.id,
                order_id: item.order_id,
                item_id: item.item_id,
                quantity: item.quantity,
                batch_id: item.batch_id,
                location: item.location,
                created_at: item.created_at,
            })
            .collect(),
    })
}

/// 把 repository 的看板聚合记录转换为 HTTP 响应。
pub(super) fn dashboard_overview_response(
    record: DashboardOverviewRecord,
) -> controller::DashboardOverviewResponse {
    controller::DashboardOverviewResponse {
        total_items: record.total_items,
        total_quantity: record.total_quantity,
        total_value: record.total_value,
        inbound_3d: record.inbound_3d,
        outbound_3d: record.outbound_3d,
        slow_moving_items: record
            .slow_moving_items
            .into_iter()
            .map(|item| controller::SlowMovingItem {
                item_id: item.item_id,
                item_name: item.item_name,
                quantity: item.quantity,
                value: item.value,
                days_since_last_movement: item.days_since_last_movement,
            })
            .collect(),
    }
}

/// 把替代料记录转换为 HTTP 响应，并保留替代物品当前库存量。
pub(super) fn substitute_response(
    record: StockSubstituteRecord,
) -> controller::SubstituteDetailResponse {
    controller::SubstituteDetailResponse {
        item_id: record.item_id,
        substitute_item_id: record.substitute_item_id,
        substitute_item_name: record.substitute_item_name,
        quantity: record.quantity,
        priority: record.priority,
        notes: record.notes,
        created_by_user_id: record.created_by_user_id,
        created_at: record.created_at,
    }
}

/// 把审计事件记录转换为 HTTP 响应；详情 JSON 解析失败时返回 JSON null。
pub(super) fn event_response(record: AuditEventRecord) -> controller::EventLogResponse {
    controller::EventLogResponse {
        id: record.id,
        timestamp: record.timestamp,
        user_id: record.user_id,
        username: record.username,
        entity_type: record.entity_type,
        entity_id: record.entity_id,
        action: record.action,
        details: record
            .details_json
            .as_deref()
            .and_then(|details| serde_json::from_str(details).ok())
            .unwrap_or(Value::Null),
    }
}
