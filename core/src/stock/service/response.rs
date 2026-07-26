//! 库存服务响应组装。
//!
//! 本模块属于 `stock` 业务服务层，负责把 repository 记录投影为库存 HTTP DTO。
//! 它不执行数据库查询，也不处理授权或路由。

use serde_json::Value;

use crate::{
    persistence::repository::{
        AuditEventRecord, DashboardOverviewRecord, InboundOrderDetail, ItemCatalogPage,
        ItemInventoryRecord, ItemOptionRecord, OutboundOrderDetail, StockFilterFieldRecord,
        StockItemListRecord, StockLocationGroupRecord, StockLocationRecord,
        StockLocationTransferRecord, StockSubstituteRecord,
    },
    stock::controller,
};

use super::{item_attributes::item_attribute_responses, StockApiError};

/// 把库存物品数据库模型转换为 HTTP 响应，不暴露软删除字段。
pub(super) fn item_editor_response(
    record: StockItemListRecord,
) -> Result<controller::ItemEditorResponse, StockApiError> {
    let item = record.item;
    Ok(controller::ItemEditorResponse {
        id: item.id,
        name: item.name,
        sku: item.sku,
        category_id: item.category_id,
        attribute_template_id: item.attribute_template_id,
        image_file_id: item.image_file_id,
        image_url: format!("/api/files/{}", item.image_file_id),
        unit: item.unit,
        description: item.description,
        default_price: item.default_price,
        reorder_point: item.reorder_point,
        attributes: item_attribute_responses(record.attributes)?,
        created_at: item.created_at,
        updated_at: item.updated_at,
    })
}

/// 把库存物品详情读取模型转换为 HTTP 响应，库存聚合只反映当前有效批次。
pub(super) fn item_inventory_response(
    detail: ItemInventoryRecord,
) -> Result<controller::ItemInventoryResponse, StockApiError> {
    let item = detail.item;
    Ok(controller::ItemInventoryResponse {
        id: item.id,
        name: item.name,
        sku: item.sku,
        unit: item.unit,
        reorder_point: item.reorder_point,
        current_quantity: detail.current_quantity,
        inventory_value: detail.inventory_value,
        stock_state: controller::ItemStockState::from_code(&detail.stock_state)?,
        batch_count: detail.batch_count,
        locations: detail
            .locations
            .into_iter()
            .map(|location| controller::ItemLocationStockResponse {
                location_id: location.location_id,
                location_name: location.location_name,
                quantity: location.quantity,
                value: location.value,
                batch_count: location.batch_count,
            })
            .collect(),
    })
}

/// 把物品目录仓储分页投影为公开目录响应。
pub(super) fn item_catalog_response(
    result: ItemCatalogPage,
    page: u64,
    page_size: u64,
) -> Result<controller::ItemCatalogPageResponse, StockApiError> {
    Ok(controller::ItemCatalogPageResponse {
        items: result
            .items
            .into_iter()
            .map(|record| {
                let item = record.item;
                Ok(controller::ItemCatalogEntryResponse {
                    id: item.id,
                    name: item.name,
                    sku: item.sku,
                    category_id: item.category_id,
                    category_name: record.category_name,
                    attribute_template_id: item.attribute_template_id,
                    image_file_id: item.image_file_id,
                    image_url: format!("/api/files/{}", item.image_file_id),
                    unit: item.unit,
                    default_price: item.default_price,
                    reorder_point: item.reorder_point,
                    catalog_attributes: record
                        .catalog_attributes
                        .into_iter()
                        .map(|attribute| {
                            Ok(controller::CatalogAttributeResponse {
                                name: attribute.name,
                                value: serde_json::from_str(&attribute.value_json)
                                    .map_err(|_| StockApiError::InvalidRequest)?,
                                unit: attribute.unit,
                            })
                        })
                        .collect::<Result<Vec<_>, StockApiError>>()?,
                    current_quantity: record.current_quantity,
                    inventory_value: record.inventory_value,
                    location_count: record.location_count,
                    batch_count: record.batch_count,
                    stock_state: controller::ItemStockState::from_code(&record.stock_state)?,
                    updated_at: item.updated_at,
                })
            })
            .collect::<Result<Vec<_>, StockApiError>>()?,
        counts: controller::ItemCatalogCountsResponse {
            total: result.counts.total,
            needs_attention: result.counts.needs_attention,
            out_of_stock: result.counts.out_of_stock,
            reorder_due: result.counts.reorder_due,
            needs_configuration: result.counts.needs_configuration,
        },
        total: result.total,
        page,
        page_size,
        total_pages: super::pagination::total_pages(result.total, page_size),
    })
}

/// 把轻量物品仓储记录转换为选择器响应。
pub(super) fn item_option_response(record: ItemOptionRecord) -> controller::ItemOptionResponse {
    controller::ItemOptionResponse {
        id: record.id,
        name: record.name,
        sku: record.sku,
        category_id: record.category_id,
        category_name: record.category_name,
        attribute_template_id: record.attribute_template_id,
        recommended_inbound_template_id: record.recommended_inbound_template_id,
        recommended_inbound_template_available: record.recommended_inbound_template_available,
        image_file_id: record.image_file_id,
        image_url: format!("/api/files/{}", record.image_file_id),
        unit: record.unit,
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

/// 把库位分组记录转换为 HTTP 响应。
pub(super) fn location_group_response(
    record: StockLocationGroupRecord,
) -> controller::LocationGroupResponse {
    controller::LocationGroupResponse {
        id: record.id,
        parent_id: record.parent_id,
        name: record.name,
        sort_order: record.sort_order,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

/// 把库位记录转换为 HTTP 响应。
pub(super) fn location_response(record: StockLocationRecord) -> controller::LocationResponse {
    controller::LocationResponse {
        id: record.id,
        group_id: record.group_id,
        group_name: record.group_name,
        name: record.name,
        notes: record.notes,
        sort_order: record.sort_order,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

/// 把移库记录转换为 HTTP 响应。
pub(super) fn location_transfer_response(
    record: StockLocationTransferRecord,
) -> controller::LocationTransferResponse {
    controller::LocationTransferResponse {
        id: record.id,
        batch_id: record.batch_id,
        item_id: record.item_id,
        from_location_id: record.from_location_id,
        to_location_id: record.to_location_id,
        quantity: record.quantity,
        notes: record.notes,
        created_by_user_id: record.created_by_user_id,
        created_at: record.created_at,
    }
}

/// 把入库单详情转换为 HTTP 响应；扩展属性 JSON 解析失败时返回 `InvalidRequest`。
pub(super) fn inbound_response(
    detail: InboundOrderDetail,
) -> Result<controller::InboundResponse, StockApiError> {
    Ok(controller::InboundResponse {
        id: detail.order.id,
        source: detail.order.source,
        status: controller::OrderStatus::from_code(&detail.order.status)?,
        submission_mode: None,
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
                    item_name: item.item_name,
                    item_sku: item.item_sku,
                    item_unit: item.item_unit,
                    item_image_file_id: item.item_image_file_id,
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    location_id: item.location_id,
                    location_name: item.location_name,
                    batch_no: item.batch_no,
                    expires_at: item.expires_at,
                    inbound_template_id: item.inbound_template_id,
                    ext_attributes: item
                        .attributes_json
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
                item_name: item.item_name,
                item_sku: item.item_sku,
                item_unit: item.item_unit,
                item_image_file_id: item.item_image_file_id,
                quantity: item.quantity,
                batch_id: item.batch_id,
                location_id: item.location_id,
                location_name: item.location_name,
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

/// 把替代料记录转换为 HTTP 响应，并保留替代物品的第一层资料和库存摘要。
pub(super) fn substitute_response(
    record: StockSubstituteRecord,
) -> Result<controller::ItemSubstituteResponse, StockApiError> {
    Ok(controller::ItemSubstituteResponse {
        item_id: record.item_id,
        substitute_item_id: record.substitute_item_id,
        substitute_item_name: record.substitute_item_name,
        substitute_item_sku: record.substitute_item_sku,
        substitute_item_category_name: record.substitute_item_category_name,
        substitute_item_image_file_id: record.substitute_item_image_file_id,
        substitute_item_image_url: format!("/api/files/{}", record.substitute_item_image_file_id),
        substitute_item_unit: record.substitute_item_unit,
        substitute_item_reorder_point: record.substitute_item_reorder_point,
        quantity: record.quantity,
        substitute_item_stock_state: controller::ItemStockState::from_code(
            &record.substitute_item_stock_state,
        )?,
        priority: record.priority,
        notes: record.notes,
        created_by_user_id: record.created_by_user_id,
        created_at: record.created_at,
    })
}

/// 把替代料关系记录转换为全量列表响应，包含主物品和替代物品展示字段。
pub(super) fn substitute_relation_response(
    record: StockSubstituteRecord,
) -> controller::SubstituteRelationResponse {
    controller::SubstituteRelationResponse {
        item_id: record.item_id,
        item_name: record.item_name,
        item_sku: record.item_sku,
        substitute_item_id: record.substitute_item_id,
        substitute_item_name: record.substitute_item_name,
        substitute_item_sku: record.substitute_item_sku,
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
