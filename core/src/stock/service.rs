//! stock 模块业务服务入口。
//!
//! 本模块属于 `stock` 业务层，负责汇总库存模板、物品、出入库、看板、替代料和事件日志用例。
//! 具体业务按子模块拆分；本模块不负责 bearer token 解析，也不直接暴露数据库表结构。

mod dashboard;
mod error;
mod events;
mod inbound;
mod items;
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
pub(crate) use items::{
    create_item, delete_item, get_item, item_filter_values, list_items, update_item,
};
pub(crate) use outbound::{
    approve_outbound, create_outbound, get_outbound, list_outbound, reject_outbound,
};
pub(crate) use pagination::PaginatedResponse;
pub(crate) use substitutes::{bind_substitutes, delete_substitute, list_substitutes};
pub(crate) use templates::{
    copy_template, create_template, delete_template, get_template, list_templates, update_template,
};
