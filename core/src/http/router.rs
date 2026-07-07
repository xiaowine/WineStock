//! core 全局 HTTP 路由装配。
//!
//! 本模块属于 `core axum library` 层，只负责拼装全局 HTTP 外壳和业务模块路由。
//! 它不直接实现 `auth`、`users` 或 `rbac` 的业务流程。

use axum::Router;

use crate::{auth, state::CoreState, users, LocalServiceBootstrap};

use super::docs;

/// 构建平台壳共用的 Axum 路由器。
pub fn build_router() -> Router {
    base_router::<()>()
}

/// 构建已接入本地存储和安全前置层运行时的 Axum 路由器。
pub fn build_router_with_local_service(local_service: &LocalServiceBootstrap) -> Router {
    let state = CoreState::from_local_service(local_service);

    // 业务路由始终挂在统一 `CoreState` 之下，避免某个局部运行时充当全局根状态。
    base_router::<CoreState>()
        .merge(auth::router())
        .merge(users::router(state.clone()))
        .with_state(state)
}

fn base_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(docs::router())
}
