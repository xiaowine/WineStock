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
    AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
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
        crate::users::controller::me
    ),
    components(schemas(
        AuthRegisterRequest,
        AuthLoginRequest,
        AuthRefreshRequest,
        AuthLogoutRequest,
        AuthUserResponse,
        AuthTokenResponse
    )),
    modifiers(&SecurityAddon),
    info(title = "WineStock API", version = "0.1.0"),
    tags(
        (name = "auth", description = "Authentication endpoints")
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
