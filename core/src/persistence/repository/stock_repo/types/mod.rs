//! 库存仓储边界模型入口。
//!
//! 本模块属于 `core` 持久化层，按库存子域拆分输入与读取模型并统一导出。
//! 它不执行数据库查询，也不拥有 HTTP DTO。

mod analytics;
mod inbound;
mod items;
mod locations;
mod outbound;
mod templates;

pub(crate) use analytics::*;
pub(crate) use inbound::*;
pub(crate) use items::*;
pub(crate) use locations::*;
pub(crate) use outbound::*;
pub(crate) use templates::*;

/// 通用分页读取结果。
pub(crate) struct Page<T> {
    /// 当前页数据。
    pub items: Vec<T>,
    /// 满足条件的总记录数。
    pub total: u64,
}

pub(super) fn validate_positive_f64(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
    }
}
