//! core 全局 HTTP 外壳入口。
//!
//! 本模块属于 `core axum library` 层，只负责 OpenAPI/Swagger 和领域路由总装配。
//! 它不实现具体业务规则，具体 endpoint 由各领域模块自行提供。

mod docs;
mod error_response;
mod health;
mod router;
mod validation;

pub use docs::{OPENAPI_JSON_PATH, SWAGGER_UI_PATH};
pub(crate) use error_response::{api_error_response, ApiErrorResponse};
pub use router::{build_router, build_router_with_local_service};
pub(crate) use validation::{ValidatedJson, ValidatedPath, ValidatedQuery};
#[cfg(test)]
#[path = "../tests/http_openapi.rs"]
mod http_openapi_tests;
