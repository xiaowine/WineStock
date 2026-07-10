//! core 全局 HTTP 路由装配。
//!
//! 本模块属于 `core axum library` 层，只负责拼装全局 HTTP 外壳和业务模块路由。
//! 它不直接实现 `auth`、`users` 或 `rbac` 的业务流程。

use super::{cors, docs, error_response, health};
use crate::{auth, state::CoreState, stock, users, LocalServiceBootstrap};
use axum::middleware;
use axum::routing::get;
use axum::Router;

/// 构建平台壳共用的 Axum 路由器。
pub fn build_router() -> Router {
    apply_global_http_middleware(base_router::<()>())
}

/// 构建已接入本地存储和安全前置层运行时的 Axum 路由器。
pub fn build_router_with_local_service(local_service: &LocalServiceBootstrap) -> Router {
    let state = CoreState::from_local_service(local_service);

    // 业务路由始终挂在统一 `CoreState` 之下，避免某个局部运行时充当全局根状态。
    let router = base_router::<CoreState>()
        .merge(auth::router())
        .merge(stock::router(state.clone()))
        .merge(users::router(state.clone()))
        .with_state(state);

    // 必须在所有业务路由 merge 完成后挂载，确保预检和错误响应都经过全局 HTTP middleware。
    apply_global_http_middleware(router)
}

fn base_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/health", get(health::health))
        .merge(docs::router())
        .fallback(error_response::not_found)
        .method_not_allowed_fallback(error_response::method_not_allowed)
}

/// 在完整 Router 外层挂载全局 HTTP middleware，避免后续 merge 的业务路由绕过 CORS。
fn apply_global_http_middleware<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(middleware::from_fn(cors::apply))
}
