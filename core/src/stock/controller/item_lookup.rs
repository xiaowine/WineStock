//! 外部物品资料查询 HTTP DTO 与 handler。
//!
//! 本模块只公开 WineStock 稳定候选资料，不暴露立创上游请求或原始响应结构。

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{http::ValidatedPath, state::CoreState, stock::service};

use super::super::service::StockApiError;

/// 立创商品中未映射为固定字段的标量参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct LcscLookupParameterResponse {
    /// 立创商品参数名称。
    pub name: String,
    /// 已归一化为文本的参数值。
    pub value: String,
}

/// 候选资料来源；作为稳定判别值公开，当前仅有立创一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ItemLookupSource {
    /// 立创商城。
    Lcsc,
}

/// 可由用户确认后填写到新建物品草稿的立创候选资料。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct LcscItemLookupResponse {
    /// 候选资料来源。
    pub source: ItemLookupSource,
    /// 规范化后的立创商品编号。
    pub product_code: String,
    /// 用于物品草稿的候选名称。
    pub name: String,
    /// 可选商品描述。
    pub description: Option<String>,
    /// 可选制造商名称。
    pub manufacturer: Option<String>,
    /// 可选制造商型号。
    pub manufacturer_part: Option<String>,
    /// 可选封装信息。
    pub footprint: Option<String>,
    /// 可选受控数据手册地址。
    pub datasheet_url: Option<String>,
    /// 已通过 Core 白名单校验、允许前端无凭据跨域读取的立创商品图。
    pub image_url: Option<String>,
    /// 有库存且存在有效阶梯价时的最小起订量参考单价。
    pub default_price: Option<f64>,
    /// 未映射为固定字段的附加标量参数。
    pub parameters: Vec<LcscLookupParameterResponse>,
}

#[utoipa::path(
    get,
    path = "/api/items/lookups/lcsc/{product_code}",
    tag = "items",
    params(("product_code" = String, Path, description = "Single LCSC product code such as C2983288")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Normalized LCSC item candidate", body = LcscItemLookupResponse),
        (status = 400, description = "Invalid LCSC product code", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "LCSC product not found", body = crate::http::ApiErrorResponse),
        (status = 429, description = "LCSC lookup concurrency limit reached", body = crate::http::ApiErrorResponse),
        (status = 502, description = "LCSC service failed or returned invalid data", body = crate::http::ApiErrorResponse),
        (status = 504, description = "LCSC lookup timed out", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询单个立创商城商品的候选资料；本接口不创建或更新物品。
pub(crate) async fn lookup_lcsc_item(
    State(state): State<CoreState>,
    ValidatedPath(product_code): ValidatedPath<String>,
) -> Result<Json<LcscItemLookupResponse>, StockApiError> {
    Ok(Json(
        service::lookup_lcsc_item(&state, &product_code).await?,
    ))
}
