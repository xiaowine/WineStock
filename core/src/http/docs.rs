//! core 全局 Debug API 文档与开发期 Swagger UI 装配。
//!
//! 本模块属于 `core axum library` 层，只负责 Debug 接口文档元信息和文档路由。
//! Release 构建不注册 OpenAPI JSON 或 Swagger UI，也不编译 Swagger UI 依赖；它不承载平台前端资源。

#[cfg(debug_assertions)]
use super::{health::HealthResponse, ApiErrorResponse};
#[cfg(debug_assertions)]
use crate::auth::{
    AuthBootstrapStatus, AuthClientKind, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest,
    AuthRegisterRequest, AuthTokenResponse, AuthUserResponse,
};
use axum::Router;
#[cfg(debug_assertions)]
use axum::{routing::get, Json};
#[cfg(debug_assertions)]
use utoipa::{
    openapi::{
        content::Content,
        path::{Operation, PathItem},
        response::Response,
        schema::Ref,
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
        RefOr,
    },
    Modify, OpenApi,
};
#[cfg(all(debug_assertions, feature = "swagger-ui"))]
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI JSON 文档的服务路径。
#[cfg(debug_assertions)]
pub const OPENAPI_JSON_PATH: &str = "/api-docs/openapi.json";

/// Debug 构建中 Swagger UI 的服务路径。
#[cfg(all(debug_assertions, feature = "swagger-ui"))]
pub const SWAGGER_UI_PATH: &str = "/swagger-ui";

// 这里集中声明接口文档元信息，具体路径由带 #[utoipa::path] 的处理函数收集。
#[cfg(debug_assertions)]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::http::health::health,
        crate::users::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::bootstrap_status,
        crate::auth::controller::refresh,
        crate::auth::controller::logout,
        crate::users::controller::me,
        crate::users::controller::change_own_password,
        crate::users::controller::list_users,
        crate::users::controller::get_user,
        crate::users::controller::delete_user,
        crate::users::controller::update_user_status,
        crate::users::controller::update_user_permissions,
        crate::users::controller::reset_user_password,
        crate::users::controller::list_permissions,
        crate::files::controller::upload_image,
        crate::files::controller::read_file,
        crate::files::controller::delete_file,
        crate::stock::controller::templates::create_item_category,
        crate::stock::controller::templates::list_item_categories,
        crate::stock::controller::templates::get_item_category,
        crate::stock::controller::templates::update_item_category,
        crate::stock::controller::templates::delete_item_category,
        crate::stock::controller::templates::create_item_attribute_template,
        crate::stock::controller::templates::list_item_attribute_templates,
        crate::stock::controller::templates::get_item_attribute_template,
        crate::stock::controller::templates::update_item_attribute_template,
        crate::stock::controller::templates::delete_item_attribute_template,
        crate::stock::controller::templates::copy_item_attribute_template,
        crate::stock::controller::templates::create_inbound_template,
        crate::stock::controller::templates::list_inbound_templates,
        crate::stock::controller::templates::get_inbound_template,
        crate::stock::controller::templates::update_inbound_template,
        crate::stock::controller::templates::delete_inbound_template,
        crate::stock::controller::templates::copy_inbound_template,
        crate::stock::controller::items::create_item,
        crate::stock::controller::items::list_items,
        crate::stock::controller::items::list_item_options,
        crate::stock::controller::items::item_filter_values,
        crate::stock::controller::items::get_item,
        crate::stock::controller::items::get_item_inventory,
        crate::stock::controller::items::list_item_batches,
        crate::stock::controller::items::update_item,
        crate::stock::controller::items::delete_item,
        crate::stock::controller::locations::list_location_group_tree,
        crate::stock::controller::locations::create_location_group,
        crate::stock::controller::locations::update_location_group,
        crate::stock::controller::locations::delete_location_group,
        crate::stock::controller::locations::list_locations,
        crate::stock::controller::locations::create_location,
        crate::stock::controller::locations::update_location,
        crate::stock::controller::locations::delete_location,
        crate::stock::controller::locations::create_location_transfer,
        crate::stock::controller::substitutes::replace_substitutes,
        crate::stock::controller::substitutes::list_substitute_relations,
        crate::stock::controller::substitutes::list_item_substitutes,
        crate::stock::controller::substitutes::delete_substitute_relation,
        crate::stock::controller::inbound::create_inbound,
        crate::stock::controller::inbound::list_inbound,
        crate::stock::controller::inbound::inbound_filter_values,
        crate::stock::controller::inbound::get_inbound,
        crate::stock::controller::inbound::approve_inbound,
        crate::stock::controller::inbound::reject_inbound,
        crate::stock::controller::outbound::create_outbound,
        crate::stock::controller::outbound::list_outbound,
        crate::stock::controller::outbound::outbound_filter_values,
        crate::stock::controller::outbound::get_outbound,
        crate::stock::controller::outbound::approve_outbound,
        crate::stock::controller::outbound::reject_outbound,
        crate::stock::controller::dashboard::dashboard_overview,
        crate::stock::controller::dashboard::dashboard_trends,
        crate::stock::controller::events::list_events
    ),
    components(schemas(
        AuthRegisterRequest,
        AuthBootstrapStatus,
        AuthClientKind,
        AuthLoginRequest,
        AuthRefreshRequest,
        AuthLogoutRequest,
        AuthUserResponse,
        AuthTokenResponse,
        ApiErrorResponse,
        HealthResponse,
        crate::files::controller::ImageFileResponse,
        crate::users::controller::UserStatus,
        crate::users::controller::UserAdminResponse,
        crate::users::controller::UserStatusUpdateRequest,
        crate::users::controller::UserPermissionsUpdateRequest,
        crate::users::controller::UserPasswordChangeRequest,
        crate::users::controller::UserPasswordResetRequest,
        crate::users::controller::PermissionResponse,
        crate::users::service::PaginatedResponse<crate::users::controller::UserAdminResponse>,
        crate::stock::controller::TemplateFieldType,
        crate::stock::controller::TemplateFieldDef,
        crate::stock::controller::TemplateCopyRequest,
        crate::stock::controller::TemplateFieldResponse,
        crate::stock::controller::ItemAttributeUnitMode,
        crate::stock::controller::ItemAttributeUnitRule,
        crate::stock::controller::ItemAttributeTemplateFieldDef,
        crate::stock::controller::ItemAttributeTemplateFieldResponse,
        crate::stock::controller::ItemCategoryCreateRequest,
        crate::stock::controller::ItemCategoryUpdateRequest,
        crate::stock::controller::ItemCategoryDeleteResponse,
        crate::stock::controller::ItemCategoryResponse,
        crate::stock::controller::ItemAttributeTemplateCreateRequest,
        crate::stock::controller::ItemAttributeTemplateUpdateRequest,
        crate::stock::controller::ItemAttributeTemplateDeleteResponse,
        crate::stock::controller::ItemAttributeTemplateResponse,
        crate::stock::controller::InboundTemplateCreateRequest,
        crate::stock::controller::InboundTemplateUpdateRequest,
        crate::stock::controller::InboundTemplateResponse,
        crate::stock::controller::ItemCreateRequest,
        crate::stock::controller::ItemAttributeRequest,
        crate::stock::controller::ItemAttributeResponse,
        crate::stock::controller::ItemUpdateRequest,
        crate::stock::controller::ItemMutationResponse,
        crate::stock::controller::ItemEditorResponse,
        crate::stock::controller::CatalogAttributeResponse,
        crate::stock::controller::ItemCatalogEntryResponse,
        crate::stock::controller::ItemCatalogCountsResponse,
        crate::stock::controller::ItemCatalogPageResponse,
        crate::stock::controller::ItemOptionResponse,
        crate::stock::controller::ItemOptionPageResponse,
        crate::stock::controller::ItemInventoryResponse,
        crate::stock::controller::ItemBatchPageResponse,
        crate::stock::controller::ItemStockState,
        crate::stock::controller::ItemStockFilter,
        crate::stock::controller::ItemCatalogSort,
        crate::stock::controller::ItemLocationStockResponse,
        crate::stock::controller::ItemBatchStockResponse,
        crate::stock::controller::LocationGroupCreateRequest,
        crate::stock::controller::LocationGroupUpdateRequest,
        crate::stock::controller::LocationGroupResponse,
        crate::stock::controller::LocationGroupTreeNode,
        crate::stock::controller::LocationCreateRequest,
        crate::stock::controller::LocationUpdateRequest,
        crate::stock::controller::LocationResponse,
        crate::stock::controller::LocationTransferCreateRequest,
        crate::stock::controller::LocationTransferResponse,
        crate::stock::controller::FilterFieldSource,
        crate::stock::controller::FilterValueType,
        crate::stock::controller::FilterValueResponse,
        crate::stock::controller::FilterFieldResponse,
        crate::stock::controller::FilterValuesResponse,
        crate::stock::controller::OrderStatus,
        crate::stock::controller::InboundItemRequest,
        crate::stock::controller::InboundCreateRequest,
        crate::stock::controller::InboundItemResponse,
        crate::stock::controller::InboundResponse,
        crate::stock::controller::OutboundItemRequest,
        crate::stock::controller::OutboundCreateRequest,
        crate::stock::controller::OutboundItemResponse,
        crate::stock::controller::OutboundResponse,
        crate::stock::controller::SlowMovingItem,
        crate::stock::controller::DashboardOverviewResponse,
        crate::stock::controller::DailyTrend,
        crate::stock::controller::TrendsResponse,
        crate::stock::controller::SubstituteReplacementItem,
        crate::stock::controller::SubstituteReplaceRequest,
        crate::stock::controller::ItemSubstituteResponse,
        crate::stock::controller::SubstituteRelationResponse,
        crate::stock::controller::EventLogResponse,
        crate::stock::service::PaginatedResponse<crate::stock::controller::InboundResponse>,
        crate::stock::service::PaginatedResponse<crate::stock::controller::OutboundResponse>,
        crate::stock::service::PaginatedResponse<crate::stock::controller::EventLogResponse>
    )),
    modifiers(&SecurityAddon),
    info(title = "WineStock API", version = "0.1.0"),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User and permission management endpoints"),
        (name = "files", description = "Controlled item and inbound image endpoints"),
        (name = "item-categories", description = "Item category endpoints"),
        (name = "item-attribute-templates", description = "Optional item attribute preset endpoints"),
        (name = "inbound-templates", description = "Inbound attribute template endpoints"),
        (name = "items", description = "Stock item endpoints"),
        (name = "locations", description = "Location group, location, and transfer endpoints"),
        (name = "inbound", description = "Inbound order endpoints"),
        (name = "outbound", description = "Outbound order endpoints"),
        (name = "stock-approvals", description = "Stock approval endpoints"),
        (name = "substitutes", description = "Substitute item management endpoints"),
        (name = "dashboard", description = "Stock dashboard endpoints"),
        (name = "events", description = "Stock event log endpoints")
    )
)]
struct ApiDoc;

#[cfg(debug_assertions)]
struct SecurityAddon;

#[cfg(debug_assertions)]
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        apply_default_bad_request_responses(openapi);
    }
}

/// 为所有接口补齐统一 400 错误响应文档，避免新增 extractor 后 OpenAPI 漏掉解析错误。
#[cfg(debug_assertions)]
fn apply_default_bad_request_responses(openapi: &mut utoipa::openapi::OpenApi) {
    for path_item in openapi.paths.paths.values_mut() {
        for operation in path_item_operations(path_item) {
            if !operation_has_parseable_input(operation) {
                continue;
            }
            operation
                .responses
                .responses
                .entry("400".to_owned())
                .or_insert_with(|| RefOr::T(api_error_response("Invalid request")));
        }
    }
}

#[cfg(debug_assertions)]
fn path_item_operations(path_item: &mut PathItem) -> impl Iterator<Item = &mut Operation> {
    [
        path_item.get.as_mut(),
        path_item.put.as_mut(),
        path_item.post.as_mut(),
        path_item.delete.as_mut(),
        path_item.options.as_mut(),
        path_item.head.as_mut(),
        path_item.patch.as_mut(),
        path_item.trace.as_mut(),
    ]
    .into_iter()
    .flatten()
}

#[cfg(debug_assertions)]
fn operation_has_parseable_input(operation: &Operation) -> bool {
    operation.request_body.is_some()
        || operation
            .parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_empty())
}

#[cfg(debug_assertions)]
fn api_error_response(description: &'static str) -> Response {
    let mut response = Response::new(description);
    response.content.insert(
        "application/json".to_owned(),
        Content::new(Some(Ref::from_schema_name("ApiErrorResponse"))),
    );
    response
}

/// Debug 构建启用 Swagger UI feature 时挂载 Swagger UI，并由其同时提供 OpenAPI JSON。
#[cfg(all(debug_assertions, feature = "swagger-ui"))]
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, ApiDoc::openapi()))
}

/// Debug 构建未启用 Swagger UI feature 时只挂载 OpenAPI JSON。
#[cfg(all(debug_assertions, not(feature = "swagger-ui")))]
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route(OPENAPI_JSON_PATH, get(openapi_json))
}

/// Release 构建不挂载 API 文档，避免把 OpenAPI 生成代码链接进最终制品。
#[cfg(not(debug_assertions))]
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
}

/// 在不依赖 Swagger UI 的情况下返回动态生成的 OpenAPI 文档。
#[cfg(debug_assertions)]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
