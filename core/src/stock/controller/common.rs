//! 库存控制器共用类型和校验函数。
//!
//! 本模块属于 `stock` HTTP 控制器层，只保存多个库存 HTTP 子模块共享的响应枚举和请求校验入口。
//! 它不访问数据库，也不承载具体业务流程。

use serde::{Deserialize, Serialize};

use crate::stock::service::StockApiError;
/// 入库单状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OrderStatus {
    /// 待审批，尚未改变库存。
    Pending,

    /// 已审批，审批事务已写入批次、库存流水和审计事件。
    Approved,

    /// 已拒绝，不能再审批。
    Rejected,
}

impl OrderStatus {
    /// 从数据库状态代码恢复 API 枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

pub(super) fn validate_positive_number(value: &f64, _: &()) -> garde::Result {
    if value.is_finite() && *value > 0.0 {
        Ok(())
    } else {
        Err(garde::Error::new("must_be_positive"))
    }
}
