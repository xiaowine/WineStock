//! 分类、物品属性模板与入库模板 HTTP 控制器入口。
//!
//! 本模块属于 stock HTTP 层，只汇总拆分后的模板接口，不重新引入统一库存模板概念。

mod categories;
mod common;
mod inbound;
mod item;

pub(crate) use categories::*;
pub(crate) use common::*;
pub(crate) use inbound::*;
pub(crate) use item::*;
