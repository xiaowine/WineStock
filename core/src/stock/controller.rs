//! stock 模块 HTTP 控制器入口。
//!
//! 本模块属于 `stock` 业务层，负责汇总库存 API 的 DTO、Axum handler 和 OpenAPI 标注。
//! 具体业务按子模块拆分，业务流程仍交给 `service`，本模块不直接访问数据库。

mod common;
pub(crate) mod dashboard;
pub(crate) mod events;
pub(crate) mod inbound;
pub(crate) mod item_attributes;
pub(crate) mod item_lookup;
pub(crate) mod items;
pub(crate) mod locations;
pub(crate) mod outbound;
pub(crate) mod substitutes;
pub(crate) mod templates;

pub(crate) use common::{
    FilterFieldResponse, FilterFieldSource, FilterValueResponse, FilterValueType,
    FilterValuesResponse, OrderStatus,
};
// 以下纯契约类型只被 Debug OpenAPI 组件注册和 #[cfg(test)] 测试点名，Release 运行时仅由父请求结构携带；
// 统一按 any(test, debug_assertions) 门控，保持 Release 构建零未使用导入告警。
#[cfg(any(test, debug_assertions))]
pub(crate) use common::{FileAttributeReference, ItemAttributeValue};
pub(crate) use dashboard::{
    dashboard_overview, dashboard_trends, DailyTrend, DashboardOverviewResponse, SlowMovingItem,
    TrendsQuery, TrendsResponse,
};
pub(crate) use events::{list_events, EventListQuery, EventLogResponse};
#[cfg(any(test, debug_assertions))]
pub(crate) use inbound::InboundItemRequest;
pub(crate) use inbound::{
    approve_inbound, create_inbound, get_inbound, inbound_filter_values, list_inbound,
    reject_inbound, InboundCreateRequest, InboundItemResponse, InboundListQuery, InboundResponse,
    InboundSubmissionMode,
};
pub(crate) use item_attributes::{ItemAttributeRequest, ItemAttributeResponse};
pub(crate) use item_lookup::{
    lookup_lcsc_item, lookup_lcsc_items, ItemLookupSource, LcscBatchLookupError,
    LcscBatchLookupResponse, LcscBatchLookupResult, LcscItemLookupResponse,
    LcscLookupParameterResponse,
};
// 供 http::docs 的 OpenAPI schema 注册通过 controller 路径引用。
#[allow(unused_imports)]
pub(crate) use item_lookup::LcscBatchLookupRequest;
pub(crate) use items::{
    create_item, delete_item, get_item, get_item_inventory, item_filter_values, list_item_batches,
    list_item_options, list_items, lookup_item_options, update_item, CatalogAttributeResponse,
    ItemBatchPageResponse, ItemBatchQuery, ItemBatchStockResponse, ItemCatalogCountsResponse,
    ItemCatalogEntryResponse, ItemCatalogFieldFilterQuery, ItemCatalogPageResponse,
    ItemCatalogQuery, ItemCatalogSort, ItemCreateRequest, ItemEditorResponse,
    ItemFilterValuesQuery, ItemInventoryResponse, ItemLocationStockResponse, ItemMutationResponse,
    ItemOptionLookupRequest, ItemOptionLookupResponse, ItemOptionLookupResult,
    ItemOptionPageResponse, ItemOptionQuery, ItemOptionResponse, ItemStockFilter, ItemStockState,
    ItemUpdateRequest,
};
pub(crate) use locations::{
    create_location, create_location_group, create_location_transfer, delete_location,
    delete_location_group, list_location_group_tree, list_locations, update_location,
    update_location_group, LocationCreateRequest, LocationGroupCreateRequest,
    LocationGroupResponse, LocationGroupTreeNode, LocationGroupUpdateRequest, LocationListQuery,
    LocationResponse, LocationTransferCreateRequest, LocationTransferResponse,
    LocationUpdateRequest,
};
#[cfg(any(test, debug_assertions))]
pub(crate) use outbound::OutboundItemRequest;
pub(crate) use outbound::{
    approve_outbound, create_outbound, get_outbound, list_outbound, outbound_filter_values,
    reject_outbound, OutboundCreateRequest, OutboundItemResponse, OutboundListQuery,
    OutboundResponse,
};
#[cfg(any(test, debug_assertions))]
pub(crate) use substitutes::SubstituteReplacementItem;
pub(crate) use substitutes::{
    delete_substitute_relation, list_item_substitutes, list_substitute_relations,
    replace_substitutes, ItemSubstituteResponse, SubstituteRelationResponse,
    SubstituteReplaceRequest,
};
pub(crate) use templates::{
    copy_item_attribute_template, create_item_attribute_template, create_item_category,
    delete_item_attribute_template, delete_item_category, get_item_attribute_template,
    get_item_category, list_item_attribute_templates, list_item_categories,
    update_item_attribute_template, update_item_category, ItemAttributeTemplateCreateRequest,
    ItemAttributeTemplateDeleteResponse, ItemAttributeTemplateFieldDef,
    ItemAttributeTemplateFieldResponse, ItemAttributeTemplateResponse,
    ItemAttributeTemplateUpdateRequest, ItemAttributeUnitMode, ItemAttributeUnitRule,
    ItemCategoryCreateRequest, ItemCategoryDeleteResponse, ItemCategoryResponse,
    ItemCategoryUpdateRequest, TemplateCopyRequest, TemplateFieldDef, TemplateFieldResponse,
    TemplateFieldType,
};
