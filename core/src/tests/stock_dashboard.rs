//! stock 模块看板接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        DashboardOverviewResponse, InboundCreateRequest, InboundItemRequest, InboundResponse,
        ItemCreateRequest, OutboundCreateRequest, OutboundItemRequest, OutboundResponse,
        TrendsResponse,
    },
    test_support::{bootstrap_location_id, json_body, login_request, seeded_app},
};

#[tokio::test]
async fn dashboard_counts_only_approved_movements_and_current_batches() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "MAIN").await;

    seed_approved_inbound(&app, &login.body.access_token, item_id, 10.0, "DASH-MAIN").await;
    seed_pending_inbound(&app, &login.body.access_token, item_id, 99.0).await;

    let approved_outbound = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 4.0),
    )
    .await;
    assert_eq!(approved_outbound.status(), StatusCode::CREATED);
    let approved_order: OutboundResponse = json_body(approved_outbound).await;
    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!(
            "/api/stock-approvals/outbound/{}/approve",
            approved_order.id
        ),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);

    let rejected_outbound = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 1.0),
    )
    .await;
    let rejected_order: OutboundResponse = json_body(rejected_outbound).await;
    let rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/reject", rejected_order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::OK);

    let overview = authorized_empty_request(
        &app,
        "GET",
        "/api/dashboard/overview",
        &login.body.access_token,
    )
    .await;
    assert_eq!(overview.status(), StatusCode::OK);
    let overview: DashboardOverviewResponse = json_body(overview).await;
    assert_eq!(overview.total_items, 1);
    assert_close(overview.total_quantity, 6.0);
    assert_close(overview.total_value, 15.0);
    assert_close(overview.inbound_3d, 10.0);
    assert_close(overview.outbound_3d, 4.0);
    assert!(overview.slow_moving_items.is_empty());

    let trends = authorized_empty_request(
        &app,
        "GET",
        "/api/dashboard/trends?days=1",
        &login.body.access_token,
    )
    .await;
    assert_eq!(trends.status(), StatusCode::OK);
    let trends: TrendsResponse = json_body(trends).await;
    assert_eq!(trends.daily.len(), 1);
    assert_close(trends.daily[0].inbound_quantity, 10.0);
    assert_close(trends.daily[0].outbound_quantity, 4.0);
}

#[tokio::test]
async fn dashboard_reports_slow_moving_items_and_requires_dashboard_read() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "SLOW").await;
    seed_approved_inbound(&app, &login.body.access_token, item_id, 3.0, "DASH-SLOW").await;
    age_item_movements(&app, item_id, 40).await;

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "dashboard-viewer", &["stock.dashboard.read"])
            .await;
    let overview =
        authorized_empty_request(&app, "GET", "/api/dashboard/overview", &viewer_token).await;
    assert_eq!(overview.status(), StatusCode::OK);
    let overview: DashboardOverviewResponse = json_body(overview).await;
    assert_eq!(overview.slow_moving_items.len(), 1);
    let slow_item = &overview.slow_moving_items[0];
    assert_eq!(slow_item.item_id, item_id);
    assert_eq!(slow_item.item_name, "Dashboard Bottle SLOW");
    assert_close(slow_item.quantity, 3.0);
    assert_close(slow_item.value, 7.5);
    assert!(slow_item.days_since_last_movement >= 30);

    let no_role_token = seed_plain_user_and_login(&app, "dashboard-no-role").await;
    let forbidden =
        authorized_empty_request(&app, "GET", "/api/dashboard/trends", &no_role_token).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

async fn seed_item(app: &crate::test_support::TestApp, access_token: &str, suffix: &str) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: format!("Dashboard Bottle {suffix}"),
            sku: format!("DASH-{suffix}"),
            category_id: None,
            attribute_template_id: None,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);
    let item: serde_json::Value = json_body(item).await;

    item["id"].as_i64().expect("item id should exist")
}

async fn seed_approved_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    quantity: f64,
    batch_no: &str,
) {
    let location_id = bootstrap_location_id(app).await;
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            source: "Dashboard Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity,
                unit_price: 2.5,
                location_id,
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some("2027-01-01".to_owned()),
                inbound_template_id: None,
                ext_attributes: None,
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(created).await;
    let approved = authorized_empty_request(
        app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
}

async fn seed_pending_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    quantity: f64,
) {
    let location_id = bootstrap_location_id(app).await;
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            source: "Pending Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity,
                unit_price: 9.9,
                location_id,
                batch_no: Some("DASH-PENDING".to_owned()),
                expires_at: Some("2027-01-01".to_owned()),
                inbound_template_id: None,
                ext_attributes: None,
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
}

fn outbound_request(item_id: i64, quantity: f64) -> OutboundCreateRequest {
    OutboundCreateRequest {
        destination: "Dashboard Cellar".to_owned(),
        notes: None,
        items: vec![OutboundItemRequest {
            item_id,
            quantity,
            batch_id: None,
            location_id: None,
        }],
    }
}

async fn age_item_movements(app: &crate::test_support::TestApp, item_id: i64, days: i64) {
    app.state
        .database()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            UPDATE stock_movements
            SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
            WHERE item_id = ?
            "#,
            vec![format!("-{days} days").into(), item_id.into()],
        ))
        .await
        .expect("movement aging should succeed");
}

async fn seed_user_with_permissions_and_login(
    app: &crate::test_support::TestApp,
    username: &str,
    permissions: &[&str],
) -> String {
    crate::test_support::seed_plain_user(app.state.database(), username, "password").await;
    let rbac = crate::persistence::repository::RbacRepository::new(app.state.database());
    let users = crate::persistence::repository::UserRepository::new(app.state.database());
    let user = users
        .find_by_username(username)
        .await
        .expect("user lookup should succeed")
        .expect("user should exist");
    for permission in permissions {
        let permission_id = rbac
            .ensure_permission(permission, None)
            .await
            .expect("permission should exist");
        rbac.assign_permission_to_user(user.id, permission_id)
            .await
            .expect("permission should assign");
    }

    login_request(app, username, "password")
        .await
        .body
        .access_token
}

async fn seed_plain_user_and_login(app: &crate::test_support::TestApp, username: &str) -> String {
    crate::test_support::seed_plain_user(app.state.database(), username, "password").await;

    login_request(app, username, "password")
        .await
        .body
        .access_token
}

async fn authorized_json_request<T: serde::Serialize>(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    access_token: &str,
    body: &T,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::from(
                    serde_json::to_vec(body).expect("body should serialize"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

async fn authorized_empty_request(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    access_token: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {actual} to be close to {expected}"
    );
}
