//! 分类与物品属性模板业务服务入口。
//!
//! 本模块属于 stock 服务层，按真实业务所有权拆分用例，不提供旧的统一模板兼容层。

mod categories;
mod common;
mod item;

pub(crate) use categories::*;
pub(crate) use item::*;
