//! 外部物品资料查询 HTTP DTO 与 handler。
//!
//! 本模块只公开 WineStock 稳定候选资料，不暴露立创上游请求或原始响应结构。

use axum::{
    body::Body,
    extract::State,
    http::{header, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{http::ValidatedPath, state::CoreState, stock::service};

use super::super::service::StockApiError;

/// 立创商品中未映射为固定字段的标量参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct LcscLookupParameterResponse {
    pub name: String,
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
    pub source: ItemLookupSource,
    pub product_code: String,
    pub name: String,
    pub description: Option<String>,
    pub manufacturer: Option<String>,
    pub manufacturer_part: Option<String>,
    pub footprint: Option<String>,
    pub datasheet_url: Option<String>,
    pub default_price: Option<f64>,
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

#[utoipa::path(
    get,
    path = "/api/items/lookups/lcsc/{product_code}/image",
    tag = "items",
    params(("product_code" = String, Path, description = "Single LCSC product code such as C2983288")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Validated LCSC product image", content_type = "image/jpeg"),
        (status = 400, description = "Invalid LCSC product code", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "LCSC product or image not found", body = crate::http::ApiErrorResponse),
        (status = 429, description = "LCSC lookup concurrency limit reached", body = crate::http::ApiErrorResponse),
        (status = 502, description = "LCSC service or image validation failed", body = crate::http::ApiErrorResponse),
        (status = 504, description = "LCSC lookup timed out", body = crate::http::ApiErrorResponse)
    )
)]
/// 读取单个立创商品的受控首图；本接口不创建 WineStock 文件对象。
pub(crate) async fn lookup_lcsc_item_image(
    State(state): State<CoreState>,
    ValidatedPath(product_code): ValidatedPath<String>,
) -> Result<Response<Body>, StockApiError> {
    let image = service::lookup_lcsc_item_image(&state, &product_code).await?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, image.mime_type)
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .body(Body::from(image.bytes))
        .expect("固定图片响应头必须有效"))
}
