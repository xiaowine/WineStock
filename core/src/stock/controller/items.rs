//! 库存物品 HTTP DTO 和 handler。
//!
//! 本模块属于 `stock` HTTP 控制器层，负责物品 CRUD、列表筛选值的请求、响应和 Axum 入口。
//! 它只调用 `service` 完成业务处理，不直接访问数据库。

use crate::validation::{validate_not_blank, validate_optional_not_blank};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    http::{ValidatedJson, ValidatedPath, ValidatedQuery},
    security::CurrentUser,
    state::CoreState,
};

use super::item_attributes::{ItemAttributeRequest, ItemAttributeResponse};
use crate::stock::service::{self, StockApiError};
/// 创建库存物品请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemCreateRequest {
    /// 物品名称，服务端会裁剪首尾空白。
    #[garde(length(utf16, min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU，未软删除记录内唯一。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联物品分类 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 可选物品属性模板 ID。
    #[garde(skip)]
    pub attribute_template_id: Option<i64>,

    /// 必选物品主图文件对象 ID；应先通过图片上传接口取得。
    #[garde(range(min = 1))]
    pub image_file_id: i64,

    /// 计量单位。
    #[garde(length(utf16, min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(
        length(utf16, min = 1, max = 1024),
        custom(validate_optional_not_blank)
    )]
    pub description: Option<String>,

    /// 参考单价，不允许为负。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点，不允许为负。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 物品固有属性；不使用模板时也可自由添加。
    #[garde(dive)]
    pub attributes: Vec<ItemAttributeRequest>,
}

/// 更新库存物品请求；字段为空表示不修改。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemUpdateRequest {
    /// 物品名称，存在时服务端会裁剪首尾空白。
    #[garde(length(utf16, min = 1, max = 128), custom(validate_optional_not_blank))]
    pub name: Option<String>,

    /// 物品 SKU，存在时未软删除记录内唯一。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_optional_not_blank))]
    pub sku: Option<String>,

    /// 关联物品分类 ID；字段缺失表示不修改，null 表示清空分类。
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub category_id: Option<Option<i64>>,

    /// 可选物品属性模板 ID；字段缺失表示不修改，null 表示取消模板。
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub attribute_template_id: Option<Option<i64>>,

    /// 新物品主图文件对象 ID；字段缺失表示保留当前图片。
    #[garde(range(min = 1))]
    pub image_file_id: Option<i64>,

    /// 计量单位。
    #[garde(length(utf16, min = 1, max = 32), custom(validate_optional_not_blank))]
    pub unit: Option<String>,

    /// 物品描述；字段缺失表示不修改，null 表示清空。
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(inner(
        length(utf16, min = 1, max = 1024),
        custom(validate_optional_not_blank)
    ))]
    pub description: Option<Option<String>>,

    /// 参考单价；字段缺失表示不修改，null 表示清空。
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub default_price: Option<Option<f64>>,

    /// 再订货点；字段缺失表示不修改，null 表示清空。
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[garde(skip)]
    pub reorder_point: Option<Option<f64>>,

    /// 物品固有属性；存在时整体替换。
    #[garde(skip)]
    pub attributes: Option<Vec<ItemAttributeRequest>>,
}

/// 把更新 JSON 中明确出现的值包装为外层 `Some`，从而区分字段缺失与 null。
fn deserialize_nullable_field<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// 物品目录库存状态筛选。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemStockFilter {
    /// 不限制库存状态。
    All,
    /// 缺货或待补货。
    NeedsAttention,
    /// 零库存。
    OutOfStock,
    /// 到达补货点。
    ReorderDue,
    /// 有库存但未设置补货点。
    NeedsConfiguration,
}

/// 物品目录排序方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemCatalogSort {
    /// 按补货处理优先级。
    ReplenishmentPriority,
    /// 按物品名称。
    Name,
    /// 按库存量升序。
    QuantityAsc,
    /// 按库存量降序。
    QuantityDesc,
    /// 按库存价值降序。
    InventoryValueDesc,
    /// 按资料更新时间降序。
    UpdatedDesc,
}

/// 服务端计算的物品库存状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemStockState {
    /// 当前库存为零。
    OutOfStock,
    /// 当前库存已到达补货点。
    ReorderDue,
    /// 当前有库存但未配置补货点。
    NeedsConfiguration,
    /// 当前库存正常。
    Normal,
}

impl ItemStockState {
    /// 从 service/repository 使用的稳定代码恢复公开枚举。
    pub(crate) fn from_code(value: &str) -> Result<Self, StockApiError> {
        match value {
            "out_of_stock" => Ok(Self::OutOfStock),
            "reorder_due" => Ok(Self::ReorderDue),
            "needs_configuration" => Ok(Self::NeedsConfiguration),
            "normal" => Ok(Self::Normal),
            _ => Err(StockApiError::InvalidRequest),
        }
    }
}

/// 物品目录分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemCatalogQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,

    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,

    /// 按物品基础字段、模板元数据和当前库存模板值模糊搜索。
    pub search: Option<String>,

    /// 按分类 ID 筛选。
    pub category_id: Option<i64>,

    /// 按物品属性模板 ID 筛选。
    pub attribute_template_id: Option<i64>,

    /// URL 编码的 JSON 结构化筛选条件。
    pub filters: Option<String>,

    /// 库存状态筛选，默认 all。
    pub stock_filter: Option<ItemStockFilter>,

    /// 排序方式，默认 replenishment_priority。
    pub sort: Option<ItemCatalogSort>,
}

/// 单个结构化筛选字段的查询输入。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub(crate) struct ItemCatalogFieldFilterQuery {
    /// 稳定字段 key，例如 `base:unit` 或 `template:42`。
    pub key: String,

    /// 同一字段内按 OR 匹配的值。
    pub values: Vec<String>,
}

/// 物品目录筛选值查询上下文。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemFilterValuesQuery {
    /// 按物品基础字段、模板元数据和物品属性模糊搜索。
    pub search: Option<String>,
    /// 按分类 ID 筛选。
    pub category_id: Option<i64>,
    /// 按物品属性模板 ID 筛选。
    pub attribute_template_id: Option<i64>,
    /// 库存状态筛选，默认 all。
    pub stock_filter: Option<ItemStockFilter>,
    /// URL 编码的 JSON 结构化筛选条件。
    pub filters: Option<String>,
}

/// 轻量物品选择分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemOptionQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,
    /// 每页数量，默认 50，最大 200。
    pub page_size: Option<u64>,
    /// 名称、SKU、分类、模板或属性搜索词。
    pub search: Option<String>,
    /// 分类 ID。
    pub category_id: Option<i64>,
    /// 物品属性模板 ID。
    pub attribute_template_id: Option<i64>,
}

/// 创建或更新物品后的轻量命令结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemMutationResponse {
    /// 已创建或更新的物品 ID。
    pub id: i64,
    /// 服务端最终资料更新时间。
    pub updated_at: String,
}

/// 物品编辑器资料响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemEditorResponse {
    /// 物品 ID。
    #[garde(skip)]
    pub id: i64,

    /// 物品名称。
    #[garde(length(utf16, min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 关联物品分类 ID。
    #[garde(skip)]
    pub category_id: Option<i64>,

    /// 可选物品属性模板 ID。
    #[garde(skip)]
    pub attribute_template_id: Option<i64>,

    /// 物品主图文件对象 ID。
    #[garde(range(min = 1))]
    pub image_file_id: i64,

    /// 物品主图受控读取地址。
    #[garde(length(bytes, min = 1, max = 256), custom(validate_not_blank))]
    pub image_url: String,

    /// 计量单位。
    #[garde(length(utf16, min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 物品描述。
    #[garde(
        length(utf16, min = 1, max = 1024),
        custom(validate_optional_not_blank)
    )]
    pub description: Option<String>,

    /// 参考单价。
    #[garde(skip)]
    pub default_price: Option<f64>,

    /// 再订货点。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 物品固有属性。
    #[garde(dive)]
    pub attributes: Vec<ItemAttributeResponse>,

    /// 创建时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub created_at: String,

    /// 最近更新时间，使用 SQLite UTC 字符串格式。
    #[garde(skip)]
    pub updated_at: String,
}

/// 目录中单个模板关键属性。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct CatalogAttributeResponse {
    /// 模板定义的属性名称。
    pub name: String,
    /// 类型化 JSON 属性值。
    #[schema(value_type = super::common::ItemAttributeValue)]
    pub value: serde_json::Value,
    /// 数字属性的实际单位。
    pub unit: Option<String>,
}

/// 物品目录单行响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemCatalogEntryResponse {
    /// 物品 ID。
    pub id: i64,
    /// 物品名称。
    pub name: String,
    /// 物品 SKU。
    pub sku: String,
    /// 分类 ID。
    pub category_id: Option<i64>,
    /// 分类名称。
    pub category_name: Option<String>,
    /// 物品属性模板 ID。
    pub attribute_template_id: Option<i64>,
    /// 主图文件 ID。
    pub image_file_id: i64,
    /// 主图受控读取地址。
    pub image_url: String,
    /// 计量单位。
    pub unit: String,
    /// 参考单价。
    pub default_price: Option<f64>,
    /// 再订货点。
    pub reorder_point: Option<f64>,
    /// 模板显式配置的目录关键属性。
    pub catalog_attributes: Vec<CatalogAttributeResponse>,
    /// 当前库存量。
    pub current_quantity: f64,
    /// 当前库存价值。
    pub inventory_value: f64,
    /// 当前有库存的库位数。
    pub location_count: u64,
    /// 当前有余额的批次数。
    pub batch_count: u64,
    /// 当前库存状态。
    pub stock_state: ItemStockState,
    /// 资料更新时间。
    pub updated_at: String,
}

/// 物品目录五项状态计数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemCatalogCountsResponse {
    /// 忽略库存状态筛选的全部物品数。
    pub total: u64,
    /// 缺货与待补货数量之和。
    pub needs_attention: u64,
    /// 缺货数量。
    pub out_of_stock: u64,
    /// 待补货数量。
    pub reorder_due: u64,
    /// 需配置补货点数量。
    pub needs_configuration: u64,
}

/// 带状态计数的物品目录分页响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemCatalogPageResponse {
    /// 当前页物品。
    pub items: Vec<ItemCatalogEntryResponse>,
    /// 状态筛选计数。
    pub counts: ItemCatalogCountsResponse,
    /// 应用当前库存状态筛选后的总数。
    pub total: u64,
    /// 当前页码。
    pub page: u64,
    /// 每页数量。
    pub page_size: u64,
    /// 总页数。
    pub total_pages: u64,
}

/// 业务选择器使用的轻量物品响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemOptionResponse {
    /// 物品 ID。
    pub id: i64,
    /// 物品名称。
    pub name: String,
    /// 物品 SKU。
    pub sku: String,
    /// 分类 ID。
    pub category_id: Option<i64>,
    /// 分类名称。
    pub category_name: Option<String>,
    /// 物品属性模板 ID。
    pub attribute_template_id: Option<i64>,
    /// 主图文件 ID。
    pub image_file_id: i64,
    /// 主图受控读取地址。
    pub image_url: String,
    /// 计量单位。
    pub unit: String,
}

/// 轻量物品选择分页响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemOptionPageResponse {
    /// 当前页物品选项。
    pub items: Vec<ItemOptionResponse>,
    /// 匹配总数。
    pub total: u64,
    /// 当前页码。
    pub page: u64,
    /// 每页数量。
    pub page_size: u64,
    /// 总页数。
    pub total_pages: u64,
}

/// 按客编批量查询库内轻量物品的请求。
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate,
)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemOptionLookupRequest {
    /// 需要精确匹配的物品 SKU，最多 500 个。
    #[garde(length(min = 1, max = 500))]
    pub product_codes: Vec<String>,
}

/// 单个客编的本地物品匹配结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemOptionLookupResult {
    /// 请求中的规范化客编。
    pub product_code: String,
    /// 命中的本地物品；未命中时为空。
    pub item: Option<ItemOptionResponse>,
    /// 结果级错误；未命中使用 `not_found`，正常命中为空。
    pub error: Option<String>,
}

/// 批量本地物品匹配响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemOptionLookupResponse {
    /// 按去重后的输入顺序返回的匹配结果。
    pub results: Vec<ItemOptionLookupResult>,
}

/// 单个已有物品的库存详情响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemInventoryResponse {
    /// 物品 ID。
    #[garde(skip)]
    pub id: i64,

    /// 物品名称。
    #[garde(length(utf16, min = 1, max = 128), custom(validate_not_blank))]
    pub name: String,

    /// 物品 SKU。
    #[garde(length(bytes, min = 1, max = 64), custom(validate_not_blank))]
    pub sku: String,

    /// 计量单位。
    #[garde(length(utf16, min = 1, max = 32), custom(validate_not_blank))]
    pub unit: String,

    /// 再订货点。
    #[garde(skip)]
    pub reorder_point: Option<f64>,

    /// 当前剩余库存总量，只统计仍有余额的批次。
    #[garde(skip)]
    pub current_quantity: f64,

    /// 当前库存价值，按批次剩余数量乘以批次单价汇总。
    #[garde(skip)]
    pub inventory_value: f64,

    /// 当前库存状态。
    #[garde(skip)]
    pub stock_state: ItemStockState,

    /// 当前有效批次数量。
    #[garde(skip)]
    pub batch_count: u64,

    /// 当前库存按库位聚合后的分布。
    #[garde(skip)]
    pub locations: Vec<ItemLocationStockResponse>,
}

/// 单个物品批次分页查询参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::IntoParams)]
pub(crate) struct ItemBatchQuery {
    /// 页码，从 1 开始，默认 1。
    pub page: Option<u64>,
    /// 每页数量，默认 20，最大 100。
    pub page_size: Option<u64>,
}

/// 单个物品当前批次分页响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ItemBatchPageResponse {
    /// 当前页批次。
    pub items: Vec<ItemBatchStockResponse>,
    /// 当前有效批次总数。
    pub total: u64,
    /// 当前页码。
    pub page: u64,
    /// 每页数量。
    pub page_size: u64,
    /// 总页数。
    pub total_pages: u64,
}

/// 物品详情中的库位库存分布。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemLocationStockResponse {
    /// 库位 ID。
    #[garde(skip)]
    pub location_id: i64,

    /// 库位名称。
    #[garde(skip)]
    pub location_name: String,

    /// 该库位当前剩余库存量。
    #[garde(skip)]
    pub quantity: f64,

    /// 该库位当前库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 该库位当前仍有余额的批次数。
    #[garde(skip)]
    pub batch_count: i64,
}

/// 物品详情中的当前批次摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub(crate) struct ItemBatchStockResponse {
    /// 批次 ID。
    #[garde(skip)]
    pub id: i64,

    /// 批次号。
    #[garde(skip)]
    pub batch_no: String,

    /// 批次库位 ID。
    #[garde(skip)]
    pub location_id: i64,

    /// 批次库位名称。
    #[garde(skip)]
    pub location_name: String,

    /// 入库时的初始数量。
    #[garde(skip)]
    pub initial_quantity: f64,

    /// 当前剩余数量。
    #[garde(skip)]
    pub remaining_quantity: f64,

    /// 批次单价。
    #[garde(skip)]
    pub unit_cost: f64,

    /// 当前批次库存价值。
    #[garde(skip)]
    pub value: f64,

    /// 入库审批时间。
    #[garde(skip)]
    pub received_at: String,

    /// 有效期。
    #[garde(skip)]
    pub expires_at: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/items",
    tag = "items",
    request_body = ItemCreateRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Item created", body = ItemMutationResponse),
        (status = 400, description = "Invalid item request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage permission required", body = crate::http::ApiErrorResponse),
        (status = 409, description = "SKU already exists", body = crate::http::ApiErrorResponse)
    )
)]
/// 创建库存物品。
pub(crate) async fn create_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedJson(request): ValidatedJson<ItemCreateRequest>,
) -> Result<(StatusCode, Json<ItemMutationResponse>), StockApiError> {
    Ok((
        StatusCode::CREATED,
        Json(service::create_item(&state, &current_user, request).await?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/items",
    tag = "items",
    params(ItemCatalogQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item catalog", body = ItemCatalogPageResponse),
        (status = 400, description = "Invalid item catalog filters", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询库存物品。
pub(crate) async fn list_items(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<ItemCatalogQuery>,
) -> Result<Json<ItemCatalogPageResponse>, StockApiError> {
    Ok(Json(service::list_item_catalog(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/options",
    tag = "items",
    params(ItemOptionQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item options", body = ItemOptionPageResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询业务选择器使用的轻量物品选项。
pub(crate) async fn list_item_options(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<ItemOptionQuery>,
) -> Result<Json<ItemOptionPageResponse>, StockApiError> {
    Ok(Json(service::list_item_options(&state, query).await?))
}

#[utoipa::path(
    post,
    path = "/api/items/options/lookup",
    tag = "items",
    request_body = ItemOptionLookupRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Batch item option lookup", body = ItemOptionLookupResponse),
        (status = 400, description = "Invalid request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 按客编批量精确查询业务选择器使用的轻量物品。
pub(crate) async fn lookup_item_options(
    State(state): State<CoreState>,
    ValidatedJson(request): ValidatedJson<ItemOptionLookupRequest>,
) -> Result<Json<ItemOptionLookupResponse>, StockApiError> {
    Ok(Json(service::lookup_item_options(&state, request).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/filter-values",
    tag = "items",
    params(ItemFilterValuesQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item catalog filter values", body = super::FilterValuesResponse),
        (status = 400, description = "Invalid item catalog filters", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询物品目录当前上下文下的分面筛选值。
pub(crate) async fn item_filter_values(
    State(state): State<CoreState>,
    ValidatedQuery(query): ValidatedQuery<ItemFilterValuesQuery>,
) -> Result<Json<super::FilterValuesResponse>, StockApiError> {
    Ok(Json(service::item_filter_values(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}",
    tag = "items",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item editor data", body = ItemEditorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询单个库存物品。
pub(crate) async fn get_item(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<ItemEditorResponse>, StockApiError> {
    Ok(Json(service::get_item(&state, id).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}/inventory",
    tag = "items",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item inventory", body = ItemInventoryResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 查询单个已有物品的库存摘要和库位分布。
pub(crate) async fn get_item_inventory(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<Json<ItemInventoryResponse>, StockApiError> {
    Ok(Json(service::get_item_inventory(&state, id).await?))
}

#[utoipa::path(
    get,
    path = "/api/items/{id}/batches",
    tag = "items",
    params(("id" = i64, Path, description = "Item ID"), ItemBatchQuery),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item batches", body = ItemBatchPageResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item read permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 分页查询单个物品当前仍有余额的批次。
pub(crate) async fn list_item_batches(
    State(state): State<CoreState>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedQuery(query): ValidatedQuery<ItemBatchQuery>,
) -> Result<Json<ItemBatchPageResponse>, StockApiError> {
    Ok(Json(service::list_item_batches(&state, id, query).await?))
}

#[utoipa::path(
    put,
    path = "/api/items/{id}",
    tag = "items",
    request_body = ItemUpdateRequest,
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Item updated", body = ItemMutationResponse),
        (status = 400, description = "Invalid item request", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "SKU already exists", body = crate::http::ApiErrorResponse)
    )
)]
/// 更新库存物品。
pub(crate) async fn update_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
    ValidatedJson(request): ValidatedJson<ItemUpdateRequest>,
) -> Result<Json<ItemMutationResponse>, StockApiError> {
    Ok(Json(
        service::update_item(&state, &current_user, id, request).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/items/{id}",
    tag = "items",
    params(("id" = i64, Path, description = "Item ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Item deleted"),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage permission required", body = crate::http::ApiErrorResponse),
        (status = 404, description = "Item not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 软删除库存物品。
pub(crate) async fn delete_item(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    ValidatedPath(id): ValidatedPath<i64>,
) -> Result<StatusCode, StockApiError> {
    service::delete_item(&state, &current_user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
