//! SeaORM Entity 模块集合。
//!
//! 这些模块只描述数据库表和字段映射，不放业务查询流程。
//! 跨表查询、事务和业务语义应放在 `persistence::repository`。

pub(crate) mod auth_setting;
pub(crate) mod auth_signing_key;

#[allow(dead_code)]
pub(crate) mod file_object;
pub(crate) mod refresh_token;
pub(crate) mod stock_item;
pub(crate) mod stock_template;
pub(crate) mod stock_template_field;
pub(crate) mod user;
