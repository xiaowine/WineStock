//! SeaORM Entity 模块集合。
//!
//! 这些模块只描述数据库表和字段映射，不放业务查询流程。
//! 跨表查询、事务和业务语义应放在 `persistence::repository`。

pub(crate) mod auth_setting;
pub(crate) mod auth_signing_key;

#[allow(dead_code)]
pub(crate) mod file_object;
#[allow(dead_code)]
pub(crate) mod inbound_file_binding;
#[allow(dead_code)]
pub(crate) mod inbound_order_item_attribute;
pub(crate) mod inbound_template;
pub(crate) mod inbound_template_field;
pub(crate) mod item_attribute;
pub(crate) mod item_attribute_template;
pub(crate) mod item_attribute_template_field;
#[allow(dead_code)]
pub(crate) mod item_file_binding;
pub(crate) mod refresh_token;
pub(crate) mod stock_item;
pub(crate) mod stock_item_category;
pub(crate) mod user;
