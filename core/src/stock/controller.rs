//! stock 模块 HTTP 控制器入口。
//!
//! 本模块属于 `stock` 业务层，负责汇总库存 API 的 DTO、Axum handler 和 OpenAPI 标注。
//! 具体业务按子模块拆分，业务流程仍交给 `service`，本模块不直接访问数据库。

mod common;
pub(crate) mod dashboard;
pub(crate) mod events;
pub(crate) mod inbound;
pub(crate) mod items;
pub(crate) mod outbound;
pub(crate) mod substitutes;
pub(crate) mod templates;

pub(crate) use common::{
    FilterFieldResponse, FilterFieldSource, FilterValueResponse, FilterValueType,
    FilterValuesResponse, OrderStatus,
};
pub(crate) use dashboard::{
    dashboard_overview, dashboard_trends, DailyTrend, DashboardOverviewResponse, SlowMovingItem,
    TrendsQuery, TrendsResponse,
};
pub(crate) use events::{list_events, EventListQuery, EventLogResponse};
pub(crate) use inbound::{
    approve_inbound, create_inbound, get_inbound, inbound_filter_values, list_inbound,
    reject_inbound, InboundCreateRequest, InboundItemRequest, InboundItemResponse,
    InboundListQuery, InboundResponse,
};
pub(crate) use items::{
    create_item, delete_item, get_item, item_filter_values, list_items, update_item,
    ItemBatchStockResponse, ItemCreateRequest, ItemDetailResponse, ItemListQuery,
    ItemLocationStockResponse, ItemResponse, ItemUpdateRequest,
};
pub(crate) use outbound::{
    approve_outbound, create_outbound, get_outbound, list_outbound, outbound_filter_values,
    reject_outbound, OutboundCreateRequest, OutboundItemRequest, OutboundItemResponse,
    OutboundListQuery, OutboundResponse,
};
pub(crate) use substitutes::{
    bind_substitutes, delete_substitute, list_substitutes, SubstituteBindRequest,
    SubstituteDetailResponse, SubstituteItem,
};
pub(crate) use templates::{
    copy_template, create_template, delete_template, get_template, list_templates, update_template,
    TemplateCopyRequest, TemplateCreateRequest, TemplateFieldDef, TemplateFieldResponse,
    TemplateFieldType, TemplateResponse, TemplateUpdateRequest,
};
