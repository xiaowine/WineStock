//! 分类、物品属性模板与入库模板业务服务入口。
//!
//! 本模块属于 stock 服务层，按真实业务所有权拆分用例，不提供旧的统一模板兼容层。

mod categories;
mod common;
mod inbound;
mod item;

pub(crate) use categories::*;
pub(crate) use inbound::*;
pub(crate) use item::*;
