//! core 全局 OpenAPI/Swagger 装配。
//!
//! 本模块属于 `core axum library` 层，只负责接口文档元信息和 Swagger UI 路由。
//! 它不决定具体业务实现，也不承载桌面或 Android 的平台前端资源。

use axum::Router;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;
use winestock_shared::{
    AuthClientKind, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};

/// OpenAPI JSON 文档的服务路径。
pub const OPENAPI_JSON_PATH: &str = "/api-docs/openapi.json";

/// Swagger UI 的服务路径。
pub const SWAGGER_UI_PATH: &str = "/swagger-ui";

// 这里集中声明接口文档元信息，具体路径由带 #[utoipa::path] 的处理函数收集。
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::users::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::refresh,
        crate::auth::controller::logout,
        crate::users::controller::me,
        crate::users::controller::change_own_password,
        crate::users::controller::list_users,
        crate::users::controller::get_user,
        crate::users::controller::update_user_status,
        crate::users::controller::update_user_permissions,
        crate::users::controller::reset_user_password,
        crate::users::controller::list_permissions,
        crate::stock::controller::templates::create_template,
        crate::stock::controller::templates::list_templates,
        crate::stock::controller::templates::get_template,
        crate::stock::controller::templates::update_template,
        crate::stock::controller::templates::delete_template,
        crate::stock::controller::templates::copy_template,
        crate::stock::controller::items::create_item,
        crate::stock::controller::items::list_items,
        crate::stock::controller::items::get_item,
        crate::stock::controller::items::update_item,
        crate::stock::controller::items::delete_item,
        crate::stock::controller::substitutes::bind_substitutes,
        crate::stock::controller::substitutes::list_substitutes,
        crate::stock::controller::substitutes::delete_substitute,
        crate::stock::controller::inbound::create_inbound,
        crate::stock::controller::inbound::list_inbound,
        crate::stock::controller::inbound::get_inbound,
        crate::stock::controller::inbound::approve_inbound,
        crate::stock::controller::inbound::reject_inbound,
        crate::stock::controller::outbound::create_outbound,
        crate::stock::controller::outbound::list_outbound,
        crate::stock::controller::outbound::get_outbound,
        crate::stock::controller::outbound::approve_outbound,
        crate::stock::controller::outbound::reject_outbound,
        crate::stock::controller::dashboard::dashboard_overview,
        crate::stock::controller::dashboard::dashboard_trends,
        crate::stock::controller::events::list_events
    ),
    components(schemas(
        AuthRegisterRequest,
        AuthClientKind,
        AuthLoginRequest,
        AuthRefreshRequest,
        AuthLogoutRequest,
        AuthUserResponse,
        AuthTokenResponse,
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
        crate::stock::controller::TemplateCreateRequest,
        crate::stock::controller::TemplateUpdateRequest,
        crate::stock::controller::TemplateCopyRequest,
        crate::stock::controller::TemplateFieldResponse,
        crate::stock::controller::TemplateResponse,
        crate::stock::controller::ItemCreateRequest,
        crate::stock::controller::ItemUpdateRequest,
        crate::stock::controller::ItemResponse,
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
        crate::stock::controller::SubstituteItem,
        crate::stock::controller::SubstituteBindRequest,
        crate::stock::controller::SubstituteDetailResponse,
        crate::stock::controller::EventLogResponse,
        crate::stock::service::PaginatedResponse<crate::stock::controller::ItemResponse>,
        crate::stock::service::PaginatedResponse<crate::stock::controller::InboundResponse>,
        crate::stock::service::PaginatedResponse<crate::stock::controller::OutboundResponse>,
        crate::stock::service::PaginatedResponse<crate::stock::controller::EventLogResponse>
    )),
    modifiers(&SecurityAddon),
    info(title = "WineStock API", version = "0.1.0"),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User and permission management endpoints")
    )
)]
struct ApiDoc;

struct SecurityAddon;

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
    }
}

/// 挂载 Swagger UI 和 OpenAPI JSON 输出。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().merge(SwaggerUi::new(SWAGGER_UI_PATH).url(OPENAPI_JSON_PATH, ApiDoc::openapi()))
}
