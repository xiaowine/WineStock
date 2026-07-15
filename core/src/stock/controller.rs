//! stock 模块 HTTP 控制器入口。
//!
//! 本模块属于 `stock` 业务层，负责汇总库存 API 的 DTO、Axum handler 和 OpenAPI 标注。
//! 具体业务按子模块拆分，业务流程仍交给 `service`，本模块不直接访问数据库。

mod common;
pub(crate) mod dashboard;
pub(crate) mod events;
pub(crate) mod inbound;
pub(crate) mod item_attributes;
pub(crate) mod items;
pub(crate) mod locations;
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
    InboundListQuery, InboundResponse, InboundSubmissionMode,
};
pub(crate) use item_attributes::{ItemAttributeRequest, ItemAttributeResponse};
pub(crate) use items::{
    create_item, delete_item, get_item, get_item_inventory, item_filter_values, list_item_batches,
    list_item_options, list_items, update_item, CatalogAttributeResponse, ItemBatchPageResponse,
    ItemBatchQuery, ItemBatchStockResponse, ItemCatalogCountsResponse, ItemCatalogEntryResponse,
    ItemCatalogFieldFilterQuery, ItemCatalogPageResponse, ItemCatalogQuery, ItemCatalogSort,
    ItemCreateRequest, ItemEditorResponse, ItemFilterValuesQuery, ItemInventoryResponse,
    ItemLocationStockResponse, ItemMutationResponse, ItemOptionPageResponse, ItemOptionQuery,
    ItemOptionResponse, ItemStockFilter, ItemStockState, ItemUpdateRequest,
};
pub(crate) use locations::{
    create_location, create_location_group, create_location_transfer, delete_location,
    delete_location_group, list_location_group_tree, list_locations, update_location,
    update_location_group, LocationCreateRequest, LocationGroupCreateRequest,
    LocationGroupResponse, LocationGroupTreeNode, LocationGroupUpdateRequest, LocationListQuery,
    LocationResponse, LocationTransferCreateRequest, LocationTransferResponse,
    LocationUpdateRequest,
};
pub(crate) use outbound::{
    approve_outbound, create_outbound, get_outbound, list_outbound, outbound_filter_values,
    reject_outbound, OutboundCreateRequest, OutboundItemRequest, OutboundItemResponse,
    OutboundListQuery, OutboundResponse,
};
pub(crate) use substitutes::{
    delete_substitute_relation, list_item_substitutes, list_substitute_relations,
    replace_substitutes, ItemSubstituteResponse, SubstituteRelationResponse,
    SubstituteReplaceRequest, SubstituteReplacementItem,
};
pub(crate) use templates::{
    copy_inbound_template, copy_item_attribute_template, create_inbound_template,
    create_item_attribute_template, create_item_category, delete_inbound_template,
    delete_item_attribute_template, delete_item_category, get_inbound_template,
    get_item_attribute_template, get_item_category, list_inbound_templates,
    list_item_attribute_templates, list_item_categories, update_inbound_template,
    update_item_attribute_template, update_item_category, InboundTemplateCreateRequest,
    InboundTemplateResponse, InboundTemplateUpdateRequest, ItemAttributeTemplateCreateRequest,
    ItemAttributeTemplateDeleteResponse, ItemAttributeTemplateFieldDef,
    ItemAttributeTemplateFieldResponse, ItemAttributeTemplateResponse,
    ItemAttributeTemplateUpdateRequest, ItemAttributeUnitMode, ItemAttributeUnitRule,
    ItemCategoryCreateRequest, ItemCategoryDeleteResponse, ItemCategoryResponse,
    ItemCategoryUpdateRequest, TemplateCopyRequest, TemplateFieldDef, TemplateFieldResponse,
    TemplateFieldType,
};
