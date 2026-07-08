//! stock 模块业务服务。
//!
//! 本模块属于 `stock` 业务层，负责库存物品用例、业务错误映射和响应组装。
//! 它不负责 bearer token 解析，也不直接暴露数据库表结构。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use winestock_shared::validation::validate_not_blank;

use crate::{
    persistence::{
        entity::stock_item,
        repository::{
            AuditEventRecord, BindStockSubstitute, CreateInboundOrder, CreateInboundOrderItem,
            CreateOutboundOrder, CreateOutboundOrderItem, CreateStockItem, CreateStockTemplate,
            DashboardOverviewRecord, InboundOrderDetail, ListAuditEvents, ListInboundOrders,
            ListOutboundOrders, ListStockItems, OutboundOrderDetail, StockRepository,
            StockSubstituteRecord, StockTemplateDetail, TemplateFieldInput, UpdateStockItem,
            UpdateStockTemplate,
        },
    },
    security::CurrentUser,
    state::CoreState,
};

/// 分页默认页码。
pub(crate) const DEFAULT_PAGE: u64 = 1;

/// 分页默认每页数量。
pub(crate) const DEFAULT_PAGE_SIZE: u64 = 50;

/// 分页最大每页数量，避免单次请求读取过多数据。
pub(crate) const MAX_PAGE_SIZE: u64 = 200;

/// 看板趋势默认天数。
pub(crate) const DEFAULT_TREND_DAYS: u64 = 30;

/// 看板趋势最大天数，避免单次返回过多图表点。
pub(crate) const MAX_TREND_DAYS: u64 = 365;

/// 呆滞料阈值天数；首版接口固定为 30 天。
pub(crate) const SLOW_MOVING_DAYS: i64 = 30;

/// 库存业务 API 错误。
#[derive(Debug)]
pub(crate) enum StockApiError {
    /// 请求字段通过 JSON 解析但不满足业务约束。
    InvalidRequest,

    /// 指定物品不存在或已软删除。
    ItemNotFound,

    /// 指定模板不存在或已软删除。
    TemplateNotFound,

    /// 指定入库单不存在。
    InboundOrderNotFound,

    /// 指定出库单不存在。
    OutboundOrderNotFound,

    /// SKU 已被其他未软删除物品占用。
    SkuTaken,

    /// 模板名称已被其他未软删除模板占用。
    TemplateNameTaken,

    /// 模板仍被未软删除物品引用，不能删除。
    TemplateInUse,

    /// 单据不是 pending 状态，不能执行审批或拒绝。
    OrderNotPending,

    /// 当前库存不足，不能审批出库单。
    InsufficientStock,

    /// 指定替代料关系不存在。
    SubstituteNotFound,

    /// 数据库读写失败。
    Database(DbErr),
}

impl IntoResponse for StockApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::ItemNotFound => (StatusCode::NOT_FOUND, "item_not_found"),
            Self::TemplateNotFound => (StatusCode::NOT_FOUND, "template_not_found"),
            Self::InboundOrderNotFound => (StatusCode::NOT_FOUND, "inbound_order_not_found"),
            Self::OutboundOrderNotFound => (StatusCode::NOT_FOUND, "outbound_order_not_found"),
            Self::SkuTaken => (StatusCode::CONFLICT, "sku_taken"),
            Self::TemplateNameTaken => (StatusCode::CONFLICT, "template_name_taken"),
            Self::TemplateInUse => (StatusCode::CONFLICT, "template_in_use"),
            Self::OrderNotPending => (StatusCode::CONFLICT, "order_not_pending"),
            Self::InsufficientStock => (StatusCode::CONFLICT, "insufficient_stock"),
            Self::SubstituteNotFound => (StatusCode::NOT_FOUND, "substitute_not_found"),
            Self::Database(source) => {
                let _ = source;
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_stock_error")
            }
        }
        .into_response()
    }
}

impl From<DbErr> for StockApiError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

/// 通用分页响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PaginatedResponse<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 满足查询条件的总记录数。
    pub total: u64,

    /// 当前页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 总页数；无数据时返回 0。
    pub total_pages: u64,
}

/// 创建库存模板。
pub(crate) async fn create_template(
    state: &CoreState,
    request: super::controller::TemplateCreateRequest,
) -> Result<super::controller::TemplateResponse, StockApiError> {
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

/// 查询库存模板列表。
pub(crate) async fn list_templates(
    state: &CoreState,
) -> Result<Vec<super::controller::TemplateResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    repository
        .list_active_templates()
        .await?
        .into_iter()
        .map(template_response)
        .collect()
}

/// 查询单个库存模板。
pub(crate) async fn get_template(
    state: &CoreState,
    id: i64,
) -> Result<super::controller::TemplateResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_active_template_by_id(id).await? else {
        return Err(StockApiError::TemplateNotFound);
    };

    template_response(detail)
}

/// 更新库存模板；字段存在时会整体替换旧字段定义。
pub(crate) async fn update_template(
    state: &CoreState,
    id: i64,
    request: super::controller::TemplateUpdateRequest,
) -> Result<super::controller::TemplateResponse, StockApiError> {
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

/// 软删除库存模板；仍有关联物品时拒绝删除。
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

/// 复制库存模板及字段定义。
pub(crate) async fn copy_template(
    state: &CoreState,
    id: i64,
    request: super::controller::TemplateCopyRequest,
) -> Result<super::controller::TemplateResponse, StockApiError> {
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

/// 创建库存物品。
pub(crate) async fn create_item(
    state: &CoreState,
    request: super::controller::ItemCreateRequest,
) -> Result<super::controller::ItemResponse, StockApiError> {
    let input = CreateStockItem {
        name: normalize_required_text(&request.name)?,
        sku: normalize_required_text(&request.sku)?,
        category_id: request.category_id,
        unit: normalize_required_text(&request.unit)?,
        description: normalize_optional_text(request.description)?,
        default_price: validate_non_negative(request.default_price)?,
        reorder_point: validate_non_negative(request.reorder_point)?,
    };
    let repository = StockRepository::new(state.database());
    if repository
        .active_sku_exists_except(&input.sku, None)
        .await?
    {
        return Err(StockApiError::SkuTaken);
    }

    Ok(item_response(repository.create_item(input).await?))
}

/// 分页查询库存物品。
pub(crate) async fn list_items(
    state: &CoreState,
    query: super::controller::ItemListQuery,
) -> Result<PaginatedResponse<super::controller::ItemResponse>, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let search = normalize_optional_text(query.search)?;
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_active_items(ListStockItems {
            page,
            page_size,
            search,
            category_id: query.category_id,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result.items.into_iter().map(item_response).collect(),
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询单个库存物品。
pub(crate) async fn get_item(
    state: &CoreState,
    id: i64,
) -> Result<super::controller::ItemResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(item) = repository.find_active_item_by_id(id).await? else {
        return Err(StockApiError::ItemNotFound);
    };

    Ok(item_response(item))
}

/// 更新库存物品基础资料。
pub(crate) async fn update_item(
    state: &CoreState,
    id: i64,
    request: super::controller::ItemUpdateRequest,
) -> Result<super::controller::ItemResponse, StockApiError> {
    let sku = request
        .sku
        .map(|sku| normalize_required_text(&sku))
        .transpose()?;
    let repository = StockRepository::new(state.database());
    if let Some(sku) = sku.as_deref() {
        if repository.active_sku_exists_except(sku, Some(id)).await? {
            return Err(StockApiError::SkuTaken);
        }
    }

    let Some(item) = repository
        .update_item(
            id,
            UpdateStockItem {
                name: request
                    .name
                    .map(|name| normalize_required_text(&name))
                    .transpose()?,
                sku,
                category_id: request.category_id.map(Some),
                unit: request
                    .unit
                    .map(|unit| normalize_required_text(&unit))
                    .transpose()?,
                description: request
                    .description
                    .map(|description| normalize_required_text(&description))
                    .transpose()?
                    .map(Some),
                default_price: request
                    .default_price
                    .map(|value| {
                        validate_non_negative(Some(value)).map(|value| value.expect("输入值已存在"))
                    })
                    .transpose()?
                    .map(Some),
                reorder_point: request
                    .reorder_point
                    .map(|value| {
                        validate_non_negative(Some(value)).map(|value| value.expect("输入值已存在"))
                    })
                    .transpose()?
                    .map(Some),
            },
        )
        .await?
    else {
        return Err(StockApiError::ItemNotFound);
    };

    Ok(item_response(item))
}

/// 软删除库存物品。
pub(crate) async fn delete_item(state: &CoreState, id: i64) -> Result<(), StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.soft_delete_item(id).await? {
        Ok(())
    } else {
        Err(StockApiError::ItemNotFound)
    }
}

/// 创建 pending 入库单；创建阶段只保存单据和明细，不改变库存数量。
pub(crate) async fn create_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    request: super::controller::InboundCreateRequest,
) -> Result<super::controller::InboundResponse, StockApiError> {
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

/// 分页查询入库单。
pub(crate) async fn list_inbound(
    state: &CoreState,
    query: super::controller::InboundListQuery,
) -> Result<PaginatedResponse<super::controller::InboundResponse>, StockApiError> {
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

/// 查询入库单详情。
pub(crate) async fn get_inbound(
    state: &CoreState,
    id: i64,
) -> Result<super::controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_inbound_order_by_id(id).await? else {
        return Err(StockApiError::InboundOrderNotFound);
    };

    inbound_response(detail)
}

/// 审批入库单；审批前按物品关联模板校验扩展属性。
pub(crate) async fn approve_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<super::controller::InboundResponse, StockApiError> {
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

/// 拒绝入库单；拒绝不写库存批次或流水。
pub(crate) async fn reject_inbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<super::controller::InboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .reject_inbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::InboundOrderNotFound)?;

    inbound_response(detail)
}

/// 创建 pending 出库单；创建阶段只保存单据和明细，不扣减库存。
pub(crate) async fn create_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    request: super::controller::OutboundCreateRequest,
) -> Result<super::controller::OutboundResponse, StockApiError> {
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
        items.push(CreateOutboundOrderItem {
            item_id: item.item_id,
            quantity: validate_positive(item.quantity)?,
            batch_id: item.batch_id,
            location: normalize_optional_text(item.location)?,
        });
    }

    let detail = repository
        .create_outbound_order(CreateOutboundOrder {
            destination: normalize_required_text(&request.destination)?,
            notes: normalize_optional_text(request.notes)?,
            created_by_user_id: Some(current_user.user_id),
            items,
        })
        .await?;

    outbound_response(detail)
}

/// 分页查询出库单。
pub(crate) async fn list_outbound(
    state: &CoreState,
    query: super::controller::OutboundListQuery,
) -> Result<PaginatedResponse<super::controller::OutboundResponse>, StockApiError> {
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
        .list_outbound_orders(ListOutboundOrders {
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
            .map(outbound_response)
            .collect::<Result<Vec<_>, StockApiError>>()?,
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

/// 查询出库单详情。
pub(crate) async fn get_outbound(
    state: &CoreState,
    id: i64,
) -> Result<super::controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let Some(detail) = repository.find_outbound_order_by_id(id).await? else {
        return Err(StockApiError::OutboundOrderNotFound);
    };

    outbound_response(detail)
}

/// 审批出库单；审批事务会按指定批次或 FIFO 扣减库存。
pub(crate) async fn approve_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<super::controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .approve_outbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::OutboundOrderNotFound)?;

    outbound_response(detail)
}

/// 拒绝出库单；拒绝不扣减库存。
pub(crate) async fn reject_outbound(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<super::controller::OutboundResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let detail = repository
        .reject_outbound_order(id, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::OutboundOrderNotFound)?;

    outbound_response(detail)
}

/// 查询库存看板总览；只读取当前库存和审批后库存流水。
pub(crate) async fn dashboard_overview(
    state: &CoreState,
) -> Result<super::controller::DashboardOverviewResponse, StockApiError> {
    let repository = StockRepository::new(state.database());
    let overview = repository.dashboard_overview(SLOW_MOVING_DAYS).await?;

    Ok(dashboard_overview_response(overview))
}

/// 查询出入库趋势；查询天数会归一到 1 到 365 天。
pub(crate) async fn dashboard_trends(
    state: &CoreState,
    query: super::controller::TrendsQuery,
) -> Result<super::controller::TrendsResponse, StockApiError> {
    let days = query
        .days
        .unwrap_or(DEFAULT_TREND_DAYS)
        .clamp(1, MAX_TREND_DAYS) as i64;
    let repository = StockRepository::new(state.database());
    let daily = repository
        .dashboard_trends(days)
        .await?
        .into_iter()
        .map(|record| super::controller::DailyTrend {
            date: record.date,
            inbound_quantity: record.inbound_quantity,
            outbound_quantity: record.outbound_quantity,
        })
        .collect();

    Ok(super::controller::TrendsResponse { daily })
}

/// 整体替换指定物品的替代料列表。
pub(crate) async fn bind_substitutes(
    state: &CoreState,
    current_user: &CurrentUser,
    item_id: i64,
    request: super::controller::SubstituteBindRequest,
) -> Result<Vec<super::controller::SubstituteDetailResponse>, StockApiError> {
    let substitutes = request
        .substitutes
        .into_iter()
        .map(|substitute| {
            Ok(BindStockSubstitute {
                substitute_item_id: positive_id(substitute.substitute_item_id)?,
                priority: positive_i32(substitute.priority)?,
                notes: normalize_optional_text(substitute.notes)?,
            })
        })
        .collect::<Result<Vec<_>, StockApiError>>()?;
    let repository = StockRepository::new(state.database());
    let records = repository
        .replace_substitutes(item_id, substitutes, Some(current_user.user_id))
        .await
        .map_err(map_stock_db_error)?
        .ok_or(StockApiError::ItemNotFound)?;

    Ok(records.into_iter().map(substitute_response).collect())
}

/// 查询指定物品的替代料列表。
pub(crate) async fn list_substitutes(
    state: &CoreState,
    item_id: i64,
) -> Result<Vec<super::controller::SubstituteDetailResponse>, StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.find_active_item_by_id(item_id).await?.is_none() {
        return Err(StockApiError::ItemNotFound);
    }

    Ok(repository
        .list_substitutes(item_id)
        .await?
        .into_iter()
        .map(substitute_response)
        .collect())
}

/// 解绑单个替代料关系。
pub(crate) async fn delete_substitute(
    state: &CoreState,
    current_user: &CurrentUser,
    item_id: i64,
    substitute_item_id: i64,
) -> Result<(), StockApiError> {
    let repository = StockRepository::new(state.database());
    if repository.find_active_item_by_id(item_id).await?.is_none() {
        return Err(StockApiError::ItemNotFound);
    }
    if !repository
        .delete_substitute(item_id, substitute_item_id, Some(current_user.user_id))
        .await?
    {
        return Err(StockApiError::SubstituteNotFound);
    }

    Ok(())
}

/// 分页查询审计事件日志。
pub(crate) async fn list_events(
    state: &CoreState,
    query: super::controller::EventListQuery,
) -> Result<PaginatedResponse<super::controller::EventLogResponse>, StockApiError> {
    let page = query.page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let repository = StockRepository::new(state.database());
    let result = repository
        .list_audit_events(ListAuditEvents {
            page,
            page_size,
            entity_type: normalize_optional_text(query.entity_type)?,
            entity_id: query.entity_id.map(positive_id).transpose()?,
            action: normalize_optional_text(query.action)?,
            user_id: query.user_id.map(positive_id).transpose()?,
            date_from: normalize_optional_text(query.date_from)?,
            date_to: normalize_optional_text(query.date_to)?,
        })
        .await?;

    Ok(PaginatedResponse {
        items: result.items.into_iter().map(event_response).collect(),
        total: result.total,
        page,
        page_size,
        total_pages: total_pages(result.total, page_size),
    })
}

fn item_response(item: stock_item::Model) -> super::controller::ItemResponse {
    super::controller::ItemResponse {
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

fn template_response(
    detail: StockTemplateDetail,
) -> Result<super::controller::TemplateResponse, StockApiError> {
    let fields = detail
        .fields
        .into_iter()
        .map(|field| {
            Ok(super::controller::TemplateFieldResponse {
                id: field.id,
                field_name: field.field_name,
                field_type: super::controller::TemplateFieldType::from_code(&field.field_type)?,
                required: sqlite_bool(field.required),
                searchable: sqlite_bool(field.searchable),
                options: parse_options_json(field.options_json)?,
                default_value: field.default_value,
                sort_order: field.sort_order,
            })
        })
        .collect::<Result<Vec<_>, StockApiError>>()?;

    Ok(super::controller::TemplateResponse {
        id: detail.template.id,
        name: detail.template.name,
        description: detail.template.description,
        fields,
        created_at: detail.template.created_at,
        updated_at: detail.template.updated_at,
    })
}

fn inbound_response(
    detail: InboundOrderDetail,
) -> Result<super::controller::InboundResponse, StockApiError> {
    Ok(super::controller::InboundResponse {
        id: detail.order.id,
        source: detail.order.source,
        status: super::controller::OrderStatus::from_code(&detail.order.status)?,
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
                Ok(super::controller::InboundItemResponse {
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

fn outbound_response(
    detail: OutboundOrderDetail,
) -> Result<super::controller::OutboundResponse, StockApiError> {
    Ok(super::controller::OutboundResponse {
        id: detail.order.id,
        destination: detail.order.destination,
        status: super::controller::OrderStatus::from_code(&detail.order.status)?,
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
            .map(|item| super::controller::OutboundItemResponse {
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

fn dashboard_overview_response(
    record: DashboardOverviewRecord,
) -> super::controller::DashboardOverviewResponse {
    super::controller::DashboardOverviewResponse {
        total_items: record.total_items,
        total_quantity: record.total_quantity,
        total_value: record.total_value,
        inbound_3d: record.inbound_3d,
        outbound_3d: record.outbound_3d,
        slow_moving_items: record
            .slow_moving_items
            .into_iter()
            .map(|item| super::controller::SlowMovingItem {
                item_id: item.item_id,
                item_name: item.item_name,
                quantity: item.quantity,
                value: item.value,
                days_since_last_movement: item.days_since_last_movement,
            })
            .collect(),
    }
}

fn substitute_response(
    record: StockSubstituteRecord,
) -> super::controller::SubstituteDetailResponse {
    super::controller::SubstituteDetailResponse {
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

fn event_response(record: AuditEventRecord) -> super::controller::EventLogResponse {
    super::controller::EventLogResponse {
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

fn parse_attribute_object(
    json: Option<&str>,
) -> Result<serde_json::Map<String, Value>, StockApiError> {
    let Some(json) = json else {
        return Ok(serde_json::Map::new());
    };
    let value: Value = serde_json::from_str(json).map_err(|_| StockApiError::InvalidRequest)?;
    value
        .as_object()
        .cloned()
        .ok_or(StockApiError::InvalidRequest)
}

fn validate_attribute_value(
    field: &crate::persistence::entity::stock_template_field::Model,
    value: &Value,
) -> Result<(), StockApiError> {
    if value.is_null() {
        return Ok(());
    }
    match super::controller::TemplateFieldType::from_code(&field.field_type)? {
        super::controller::TemplateFieldType::Text
        | super::controller::TemplateFieldType::Date
        | super::controller::TemplateFieldType::File => match value.as_str() {
            Some(text) if !text.trim().is_empty() => Ok(()),
            _ => Err(StockApiError::InvalidRequest),
        },
        super::controller::TemplateFieldType::Number => match value.as_f64() {
            Some(number) if number.is_finite() => Ok(()),
            _ => Err(StockApiError::InvalidRequest),
        },
        super::controller::TemplateFieldType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        super::controller::TemplateFieldType::Select => {
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

fn is_empty_attribute_value(value: &Value) -> bool {
    value.is_null() || matches!(value.as_str(), Some(text) if text.trim().is_empty())
}

fn normalize_template_fields(
    fields: Vec<super::controller::TemplateFieldDef>,
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

fn normalize_field_options(
    field_type: super::controller::TemplateFieldType,
    options: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, StockApiError> {
    match field_type {
        super::controller::TemplateFieldType::Select => {
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

fn validate_field_default(
    field_type: super::controller::TemplateFieldType,
    default_value: Option<&str>,
    options: Option<&[String]>,
) -> Result<(), StockApiError> {
    let Some(default_value) = default_value else {
        return Ok(());
    };

    match field_type {
        super::controller::TemplateFieldType::Number => default_value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(|_| ())
            .ok_or(StockApiError::InvalidRequest),
        super::controller::TemplateFieldType::Boolean => {
            if matches!(default_value, "true" | "false") {
                Ok(())
            } else {
                Err(StockApiError::InvalidRequest)
            }
        }
        super::controller::TemplateFieldType::Select => {
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

fn parse_options_json(value: Option<String>) -> Result<Option<Vec<String>>, StockApiError> {
    value
        .map(|value| serde_json::from_str(&value).map_err(|_| StockApiError::InvalidRequest))
        .transpose()
}

fn normalize_required_text(value: &str) -> Result<String, StockApiError> {
    validate_not_blank(value, &()).map_err(|_| StockApiError::InvalidRequest)?;
    Ok(value.trim().to_owned())
}

fn sqlite_bool(value: i32) -> bool {
    value != 0
}

fn normalize_optional_text(value: Option<String>) -> Result<Option<String>, StockApiError> {
    value
        .map(|value| normalize_required_text(&value))
        .transpose()
}

fn validate_non_negative(value: Option<f64>) -> Result<Option<f64>, StockApiError> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        Err(StockApiError::InvalidRequest)
    } else {
        Ok(value)
    }
}

fn validate_positive(value: f64) -> Result<f64, StockApiError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

fn positive_id(value: i64) -> Result<i64, StockApiError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

fn positive_i32(value: i32) -> Result<i32, StockApiError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(StockApiError::InvalidRequest)
    }
}

fn map_stock_db_error(source: DbErr) -> StockApiError {
    match &source {
        DbErr::Custom(message)
            if message == "inbound order is not pending"
                || message == "outbound order is not pending" =>
        {
            StockApiError::OrderNotPending
        }
        DbErr::Custom(message) if message == "insufficient stock" => {
            StockApiError::InsufficientStock
        }
        DbErr::Custom(message) if message == "substitute item not found" => {
            StockApiError::ItemNotFound
        }
        DbErr::Custom(message)
            if message == "substitute self reference"
                || message == "duplicate substitute item"
                || message == "duplicate substitute priority"
                || message == "substitute cycle" =>
        {
            StockApiError::InvalidRequest
        }
        _ => StockApiError::Database(source),
    }
}

fn total_pages(total: u64, page_size: u64) -> u64 {
    if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    }
}
