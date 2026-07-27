//! stock 模块业务服务入口。
//!
//! 本模块属于 `stock` 业务层，负责汇总库存模板、物品、出入库、看板、替代料和事件日志用例。
//! 具体业务按子模块拆分；本模块不负责 bearer token 解析，也不直接暴露数据库表结构。

mod dashboard;
mod error;
mod events;
mod inbound;
mod item_attributes;
mod item_lookup;
mod items;
mod locations;
mod outbound;
mod pagination;
mod response;
mod substitutes;
mod templates;
mod validation;

pub(crate) use dashboard::{dashboard_overview, dashboard_trends};
pub(crate) use error::StockApiError;
pub(crate) use events::list_events;
pub(crate) use inbound::{
    approve_inbound, create_inbound, get_inbound, inbound_filter_values, list_inbound,
    reject_inbound,
};
pub(crate) use item_lookup::lookup_lcsc_item;
pub(crate) use items::{
    create_item, delete_item, get_item, get_item_inventory, item_filter_values, list_item_batches,
    list_item_catalog, list_item_options, update_item,
};
pub(crate) use locations::{
    create_location, create_location_group, create_location_transfer, delete_location,
    delete_location_group, list_location_group_tree, list_locations, update_location,
    update_location_group,
};
pub(crate) use outbound::{
    approve_outbound, create_outbound, get_outbound, list_outbound, outbound_filter_values,
    reject_outbound,
};
pub(crate) use pagination::PaginatedResponse;
pub(crate) use substitutes::{
    delete_substitute_relation, list_item_substitutes, list_substitute_relations,
    replace_substitutes,
};
pub(crate) use templates::{
    copy_item_attribute_template, create_item_attribute_template, create_item_category,
    delete_item_attribute_template, delete_item_category, get_item_attribute_template,
    get_item_category, list_item_attribute_templates, list_item_categories,
    update_item_attribute_template, update_item_category,
};
