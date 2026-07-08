//! 库存服务分页模型。
//!
//! 本模块属于 `stock` 业务服务层，负责库存接口统一分页默认值、分页响应和页数计算。
//! 它不访问数据库，也不决定具体业务查询条件。

use serde::{Deserialize, Serialize};

/// 分页默认页码。
pub(super) const DEFAULT_PAGE: u64 = 1;

/// 分页默认每页数量。
pub(super) const DEFAULT_PAGE_SIZE: u64 = 50;

/// 分页最大每页数量，避免单次请求读取过多数据。
pub(super) const MAX_PAGE_SIZE: u64 = 200;

/// 通用分页响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct PaginatedResponse<T> {
    /// 当前页数据。
    pub items: Vec<T>,

    /// 满足查询条件的总记录数。
    pub total: u64,

    /// 当前页码，从 1 开始。
    pub page: u64,

    /// 每页数量。
    pub page_size: u64,

    /// 总页数；无数据时返回 0。
    pub total_pages: u64,
}

/// 根据总记录数和每页数量计算总页数；空结果返回 0。
pub(super) fn total_pages(total: u64, page_size: u64) -> u64 {
    if total == 0 {
        0
    } else {
        total.div_ceil(page_size)
    }
}
