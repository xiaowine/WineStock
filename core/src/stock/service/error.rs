//! 库存服务错误映射。
//!
//! 本模块属于 `stock` 业务服务层，负责库存业务错误枚举、HTTP 响应映射和 repository 自定义错误收敛。
//! 它不访问数据库，也不承载具体库存用例。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

use crate::http::{api_error_response, api_error_response_with_details};

/// 库存业务 API 错误。
#[derive(Debug)]
pub(crate) enum StockApiError {
    /// 请求字段通过 JSON 解析但不满足业务约束。
    InvalidRequest,

    /// 指定物品不存在或已软删除。
    ItemNotFound,

    /// 指定模板不存在或已软删除。
    TemplateNotFound,

    /// 指定物品分类不存在或已软删除。
    CategoryNotFound,

    /// 指定入库单不存在。
    InboundOrderNotFound,

    /// 指定出库单不存在。
    OutboundOrderNotFound,

    /// SKU 已被其他未软删除物品占用。
    SkuTaken,

    /// 立创商品编号不符合 C + 数字格式。
    InvalidLcscProductCode,

    /// 立创上游没有返回精确匹配的商品。
    LcscProductNotFound,

    /// 立创查询已达到进程并发上限。
    LcscLookupBusy,

    /// 立创查询连接或读取超时。
    LcscLookupTimeout,

    /// 立创查询网络或 HTTP 状态失败。
    LcscLookupFailed,

    /// 立创响应超限、损坏或不符合预期结构。
    LcscInvalidResponse,

    /// 物品主图不存在、无权使用、内容损坏或已经被其它业务记录绑定。
    ItemImageUnavailable { file_id: i64 },

    /// 模板名称已被其他未软删除模板占用。
    TemplateNameTaken,

    /// 分类名称已被其他未软删除分类占用。
    CategoryNameTaken,

    /// 单据不是 pending 状态，不能执行审批或拒绝。
    OrderNotPending,

    /// 请求直接入库，但当前用户没有入库审核权限。
    DirectInboundApprovalForbidden,

    /// 当前库存不足，不能审批出库单。
    InsufficientStock,

    /// 指定替代料关系不存在。
    SubstituteNotFound,

    /// 指定库位分组不存在或已删除。
    LocationGroupNotFound,

    /// 指定库位不存在或已删除。
    LocationNotFound,

    /// 库位分组同级名称已存在。
    LocationGroupNameTaken,

    /// 库位名称已被其它未删除库位占用。
    LocationNameTaken,

    /// 库位分组仍有子分组或库位，不能删除。
    LocationGroupInUse,

    /// 库位仍有当前库存，不能删除。
    LocationInUse,

    /// 库位分组移动会形成循环层级。
    LocationGroupCycle,

    /// 库位分组创建或移动后会超过十层。
    LocationGroupDepthExceeded,

    /// 指定批次不存在、无剩余库存或不满足移库条件。
    StockBatchNotFound,

    /// 创建入库单时某条明细引用的物品已经失效。
    InboundItemInvalid { line_index: usize, item_id: i64 },

    /// 创建入库单时某条明细引用的库位已经失效。
    InboundLocationInvalid { line_index: usize, location_id: i64 },

    /// 数据库读写失败。
    Database(DbErr),
}

impl IntoResponse for StockApiError {
    // 将库存业务错误固定映射为 HTTP 状态码和稳定错误代码，避免 controller 分散处理。
    fn into_response(self) -> Response {
        match self {
            Self::InboundItemInvalid {
                line_index,
                item_id,
            } => {
                return api_error_response_with_details(
                    StatusCode::NOT_FOUND,
                    "item_not_found",
                    "入库明细中的物品不存在或已失效",
                    serde_json::json!({ "line_index": line_index, "item_id": item_id }),
                );
            }
            Self::InboundLocationInvalid {
                line_index,
                location_id,
            } => {
                return api_error_response_with_details(
                    StatusCode::NOT_FOUND,
                    "location_not_found",
                    "入库明细中的库位不存在或已失效",
                    serde_json::json!({ "line_index": line_index, "location_id": location_id }),
                );
            }
            Self::ItemImageUnavailable { file_id } => {
                return api_error_response_with_details(
                    StatusCode::CONFLICT,
                    "item_image_unavailable",
                    "物品主图不存在、无权使用或已被占用",
                    serde_json::json!({ "file_id": file_id }),
                );
            }
            other => return other.into_plain_response(),
        }
    }
}

impl StockApiError {
    fn into_plain_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request", "请求参数无效"),
            Self::ItemNotFound => (StatusCode::NOT_FOUND, "item_not_found", "物品不存在"),
            Self::TemplateNotFound => (StatusCode::NOT_FOUND, "template_not_found", "模板不存在"),
            Self::CategoryNotFound => (
                StatusCode::NOT_FOUND,
                "category_not_found",
                "物品分类不存在",
            ),
            Self::InboundOrderNotFound => (
                StatusCode::NOT_FOUND,
                "inbound_order_not_found",
                "入库单不存在",
            ),
            Self::OutboundOrderNotFound => (
                StatusCode::NOT_FOUND,
                "outbound_order_not_found",
                "出库单不存在",
            ),
            Self::SkuTaken => (StatusCode::CONFLICT, "sku_taken", "SKU 已存在"),
            Self::InvalidLcscProductCode => (
                StatusCode::BAD_REQUEST,
                "invalid_lcsc_product_code",
                "立创商品编号格式无效",
            ),
            Self::LcscProductNotFound => (
                StatusCode::NOT_FOUND,
                "lcsc_product_not_found",
                "未查询到该立创商品",
            ),
            Self::LcscLookupBusy => (
                StatusCode::TOO_MANY_REQUESTS,
                "lcsc_lookup_busy",
                "立创资料查询繁忙",
            ),
            Self::LcscLookupTimeout => (
                StatusCode::GATEWAY_TIMEOUT,
                "lcsc_lookup_timeout",
                "立创资料查询超时",
            ),
            Self::LcscLookupFailed => (
                StatusCode::BAD_GATEWAY,
                "lcsc_lookup_failed",
                "暂时无法查询立创资料",
            ),
            Self::LcscInvalidResponse => (
                StatusCode::BAD_GATEWAY,
                "lcsc_invalid_response",
                "立创返回了无法识别的数据",
            ),
            Self::TemplateNameTaken => (
                StatusCode::CONFLICT,
                "template_name_taken",
                "模板名称已存在",
            ),
            Self::CategoryNameTaken => (
                StatusCode::CONFLICT,
                "category_name_taken",
                "物品分类名称已存在",
            ),
            Self::OrderNotPending => (
                StatusCode::CONFLICT,
                "order_not_pending",
                "单据不是待审批状态",
            ),
            Self::DirectInboundApprovalForbidden => (
                StatusCode::FORBIDDEN,
                "inbound_direct_approval_forbidden",
                "直接入库需要入库审核权限",
            ),
            Self::InsufficientStock => (StatusCode::CONFLICT, "insufficient_stock", "库存不足"),
            Self::SubstituteNotFound => (
                StatusCode::NOT_FOUND,
                "substitute_not_found",
                "替代料关系不存在",
            ),
            Self::LocationGroupNotFound => (
                StatusCode::NOT_FOUND,
                "location_group_not_found",
                "库位分组不存在",
            ),
            Self::LocationNotFound => (StatusCode::NOT_FOUND, "location_not_found", "库位不存在"),
            Self::LocationGroupNameTaken => (
                StatusCode::CONFLICT,
                "location_group_name_taken",
                "库位分组名称已存在",
            ),
            Self::LocationNameTaken => (
                StatusCode::CONFLICT,
                "location_name_taken",
                "库位名称已存在",
            ),
            Self::LocationGroupInUse => (
                StatusCode::CONFLICT,
                "location_group_in_use",
                "库位分组正在使用中",
            ),
            Self::LocationInUse => (StatusCode::CONFLICT, "location_in_use", "库位正在使用中"),
            Self::LocationGroupCycle => (
                StatusCode::BAD_REQUEST,
                "location_group_cycle",
                "库位分组不能移动到自己的子级",
            ),
            Self::LocationGroupDepthExceeded => (
                StatusCode::BAD_REQUEST,
                "location_group_depth_exceeded",
                "库位分组最多只能有 10 层",
            ),
            Self::StockBatchNotFound => (
                StatusCode::NOT_FOUND,
                "stock_batch_not_found",
                "库存批次不存在",
            ),
            Self::InboundItemInvalid { .. }
            | Self::InboundLocationInvalid { .. }
            | Self::ItemImageUnavailable { .. } => unreachable!("结构化库存错误已提前处理"),
            Self::Database(source) => {
                let _ = source;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_stock_error",
                    "库存服务内部错误",
                )
            }
        };

        api_error_response(status, code, message)
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
        DbErr::Custom(message) if message == "stock location not found" => {
            StockApiError::LocationNotFound
        }
        DbErr::Custom(message) if message == "stock batch not found" => {
            StockApiError::StockBatchNotFound
        }
        DbErr::Custom(message) if message == "item file unavailable" => {
            StockApiError::InvalidRequest
        }
        DbErr::Custom(message) if message.starts_with("item image unavailable:") => {
            let file_id = message
                .split_once(':')
                .and_then(|(_, value)| value.parse().ok())
                .unwrap_or(0);
            StockApiError::ItemImageUnavailable { file_id }
        }
        DbErr::Custom(message)
            if message == "location transfer target unchanged"
                || message == "location transfer source mismatch" =>
        {
            StockApiError::InvalidRequest
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
