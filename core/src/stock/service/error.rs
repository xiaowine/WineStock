//! 库存服务错误映射。
//!
//! 本模块属于 `stock` 业务服务层，负责库存业务错误枚举、HTTP 响应映射和 repository 自定义错误收敛。
//! 它不访问数据库，也不承载具体库存用例。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

/// 库存业务 API 错误。
#[derive(Debug)]
pub(crate) enum StockApiError {
    /// 请求字段通过 JSON 解析但不满足业务约束。
    InvalidRequest,

    /// 指定物品不存在或已软删除。
    ItemNotFound,

    /// 指定模板不存在或已软删除。
    TemplateNotFound,

    /// 指定入库单不存在。
    InboundOrderNotFound,

    /// 指定出库单不存在。
    OutboundOrderNotFound,

    /// SKU 已被其他未软删除物品占用。
    SkuTaken,

    /// 模板名称已被其他未软删除模板占用。
    TemplateNameTaken,

    /// 模板仍被未软删除物品引用，不能删除。
    TemplateInUse,

    /// 单据不是 pending 状态，不能执行审批或拒绝。
    OrderNotPending,

    /// 当前库存不足，不能审批出库单。
    InsufficientStock,

    /// 指定替代料关系不存在。
    SubstituteNotFound,

    /// 数据库读写失败。
    Database(DbErr),
}

impl IntoResponse for StockApiError {
    // 将库存业务错误固定映射为 HTTP 状态码和稳定错误代码，避免 controller 分散处理。
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::ItemNotFound => (StatusCode::NOT_FOUND, "item_not_found"),
            Self::TemplateNotFound => (StatusCode::NOT_FOUND, "template_not_found"),
            Self::InboundOrderNotFound => (StatusCode::NOT_FOUND, "inbound_order_not_found"),
            Self::OutboundOrderNotFound => (StatusCode::NOT_FOUND, "outbound_order_not_found"),
            Self::SkuTaken => (StatusCode::CONFLICT, "sku_taken"),
            Self::TemplateNameTaken => (StatusCode::CONFLICT, "template_name_taken"),
            Self::TemplateInUse => (StatusCode::CONFLICT, "template_in_use"),
            Self::OrderNotPending => (StatusCode::CONFLICT, "order_not_pending"),
            Self::InsufficientStock => (StatusCode::CONFLICT, "insufficient_stock"),
            Self::SubstituteNotFound => (StatusCode::NOT_FOUND, "substitute_not_found"),
            Self::Database(source) => {
                let _ = source;
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_stock_error")
            }
        }
        .into_response()
    }
}

impl From<DbErr> for StockApiError {
    // 默认数据库错误不向调用方暴露内部细节；特定业务错误由 map_stock_db_error 单独转换。
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

/// 把 repository 事务内的自定义数据库错误收敛为库存 API 错误。
pub(super) fn map_stock_db_error(source: DbErr) -> StockApiError {
    match &source {
        DbErr::Custom(message)
            if message == "inbound order is not pending"
                || message == "outbound order is not pending" =>
        {
            StockApiError::OrderNotPending
        }
        DbErr::Custom(message) if message == "insufficient stock" => {
            StockApiError::InsufficientStock
        }
        DbErr::Custom(message) if message == "substitute item not found" => {
            StockApiError::ItemNotFound
        }
        DbErr::Custom(message)
            if message == "substitute self reference"
                || message == "duplicate substitute item"
                || message == "duplicate substitute priority"
                || message == "substitute cycle" =>
        {
            StockApiError::InvalidRequest
        }
        _ => StockApiError::Database(source),
    }
}
