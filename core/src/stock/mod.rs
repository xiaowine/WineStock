//! stock 库存业务模块入口。
//!
//! 本模块属于 `core axum library` 的业务层，负责库存、模板、出入库、替代料和看板接口。
//! 它不拥有平台 shell、WebView、前端资源或服务端进程生命周期。

use axum::{
    routing::MethodRouter,
    routing::{delete, get, post, put},
    Router,
};

use crate::{security::AuthorizeRouteExt, state::CoreState};

mod bootstrap;
pub(crate) mod controller;
pub(crate) mod permissions;
pub(crate) mod service;

pub(crate) use bootstrap::bootstrap_default_templates;
pub use bootstrap::StockBootstrapError;
pub(crate) use permissions::{
    AUDIT_READ_PERMISSION, STOCK_INBOUND_APPROVE_PERMISSION, STOCK_INBOUND_CREATE_PERMISSION,
    STOCK_ITEM_MANAGE_PERMISSION, STOCK_OUTBOUND_APPROVE_PERMISSION,
    STOCK_OUTBOUND_CREATE_PERMISSION, STOCK_READ_PERMISSION, STOCK_SUBSTITUTE_MANAGE_PERMISSION,
    STOCK_TEMPLATE_MANAGE_PERMISSION, STOCK_WRITE_PERMISSION,
};

const STOCK_BASE_PATH: &str = "/api";

/// 注册库存业务 HTTP 路由集合。
pub(crate) fn router(state: CoreState) -> Router<CoreState> {
    let auth = StockRouteAuth::new(state);

    // 库存接口统一挂载在 API base path，子路由只声明领域内相对路径。
    Router::new().nest(
        STOCK_BASE_PATH,
        Router::new()
            .route(
                "/templates",
                auth.template_manage(post(controller::create_template))
                    .merge(auth.read(get(controller::list_templates))),
            )
            .route(
                "/templates/{id}",
                auth.read(get(controller::get_template))
                    .merge(auth.template_manage(put(controller::update_template)))
                    .merge(auth.template_manage(delete(controller::delete_template))),
            )
            .route(
                "/templates/{id}/copy",
                auth.template_manage(post(controller::copy_template)),
            )
            .route(
                "/items",
                auth.item_manage(post(controller::create_item))
                    .merge(auth.read(get(controller::list_items))),
            )
            .route(
                "/items/filter-values",
                auth.read(get(controller::item_filter_values)),
            )
            .route(
                "/items/{id}",
                auth.read(get(controller::get_item))
                    .merge(auth.item_manage(put(controller::update_item)))
                    .merge(auth.item_manage(delete(controller::delete_item))),
            )
            .route(
                "/items/{id}/substitutes",
                auth.substitute_manage(post(controller::bind_substitutes))
                    .merge(auth.read(get(controller::list_substitutes))),
            )
            .route(
                "/items/{id}/substitutes/{substitute_id}",
                auth.substitute_manage(delete(controller::delete_substitute)),
            )
            .route(
                "/inbound",
                auth.inbound_create(post(controller::create_inbound))
                    .merge(auth.read(get(controller::list_inbound))),
            )
            .route(
                "/inbound/filter-values",
                auth.read(get(controller::inbound_filter_values)),
            )
            .route("/inbound/{id}", auth.read(get(controller::get_inbound)))
            .route(
                "/stock-approvals/inbound/{id}/approve",
                auth.inbound_approve(post(controller::approve_inbound)),
            )
            .route(
                "/stock-approvals/inbound/{id}/reject",
                auth.inbound_approve(post(controller::reject_inbound)),
            )
            .route(
                "/outbound",
                auth.outbound_create(post(controller::create_outbound))
                    .merge(auth.read(get(controller::list_outbound))),
            )
            .route(
                "/outbound/filter-values",
                auth.read(get(controller::outbound_filter_values)),
            )
            .route("/outbound/{id}", auth.read(get(controller::get_outbound)))
            .route(
                "/stock-approvals/outbound/{id}/approve",
                auth.outbound_approve(post(controller::approve_outbound)),
            )
            .route(
                "/stock-approvals/outbound/{id}/reject",
                auth.outbound_approve(post(controller::reject_outbound)),
            )
            .route(
                "/dashboard/overview",
                auth.read(get(controller::dashboard_overview)),
            )
            .route(
                "/dashboard/trends",
                auth.read(get(controller::dashboard_trends)),
            )
            .route("/events", auth.audit_read(get(controller::list_events))),
    )
}

#[derive(Clone)]
struct StockRouteAuth {
    state: CoreState,
}

impl StockRouteAuth {
    /// 创建库存路由授权辅助对象，让路由声明只表达业务权限语义。
    fn new(state: CoreState) -> Self {
        Self { state }
    }

    fn read(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_READ_PERMISSION)
    }

    fn item_manage(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_ITEM_MANAGE_PERMISSION)
    }

    fn template_manage(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_TEMPLATE_MANAGE_PERMISSION)
    }

    fn inbound_create(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_INBOUND_CREATE_PERMISSION)
    }

    fn inbound_approve(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_INBOUND_APPROVE_PERMISSION)
    }

    fn outbound_create(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_OUTBOUND_CREATE_PERMISSION)
    }

    fn outbound_approve(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_OUTBOUND_APPROVE_PERMISSION)
    }

    fn substitute_manage(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, STOCK_SUBSTITUTE_MANAGE_PERMISSION)
    }

    fn audit_read(&self, route: MethodRouter<CoreState>) -> MethodRouter<CoreState> {
        self.allow(route, AUDIT_READ_PERMISSION)
    }

    fn allow(
        &self,
        route: MethodRouter<CoreState>,
        permission: &'static str,
    ) -> MethodRouter<CoreState> {
        route.require_permission(self.state.clone(), permission)
    }
}

#[cfg(test)]
#[path = "../tests/stock_items.rs"]
mod stock_items_tests;

#[cfg(test)]
#[path = "../tests/stock_templates.rs"]
mod stock_templates_tests;

#[cfg(test)]
#[path = "../tests/stock_inbound.rs"]
mod stock_inbound_tests;

#[cfg(test)]
#[path = "../tests/stock_outbound.rs"]
mod stock_outbound_tests;

#[cfg(test)]
#[path = "../tests/stock_dashboard.rs"]
mod stock_dashboard_tests;

#[cfg(test)]
#[path = "../tests/stock_substitutes.rs"]
mod stock_substitutes_tests;

#[cfg(test)]
#[path = "../tests/stock_events.rs"]
mod stock_events_tests;
