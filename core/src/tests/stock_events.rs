//! stock 模块事件日志接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    stock::{
        controller::{
            EventLogResponse, InboundCreateRequest, InboundItemRequest, InboundResponse,
            ItemCreateRequest,
        },
        service::PaginatedResponse,
    },
    test_support::{bootstrap_location_id, json_body, login_request, seeded_app},
};

#[tokio::test]
async fn events_can_be_filtered_and_paginated_by_audit_readers() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "EVENT").await;

    let order = seed_pending_inbound(&app, &login.body.access_token, item_id).await;
    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "event-viewer", &["audit.read"]).await;
    let events = authorized_empty_request(
        &app,
        "GET",
        &format!(
            "/api/events?entity_type=inbound&action=approved&entity_id={}&page_size=1",
            order.id
        ),
        &viewer_token,
    )
    .await;
    assert_eq!(events.status(), StatusCode::OK);
    let events: PaginatedResponse<EventLogResponse> = json_body(events).await;
    assert_eq!(events.total, 1);
    assert_eq!(events.page, 1);
    assert_eq!(events.page_size, 1);
    assert_eq!(events.total_pages, 1);
    assert_eq!(events.items.len(), 1);
    let event = &events.items[0];
    assert_eq!(event.entity_type, "inbound");
    assert_eq!(event.entity_id, Some(order.id));
    assert_eq!(event.action, "approved");
    assert_eq!(event.username.as_deref(), Some("admin"));
    assert_eq!(event.details["item_count"], 1);

    let all_inbound = authorized_empty_request(
        &app,
        "GET",
        "/api/events?entity_type=inbound&page_size=1",
        &viewer_token,
    )
    .await;
    let all_inbound: PaginatedResponse<EventLogResponse> = json_body(all_inbound).await;
    assert_eq!(all_inbound.total, 2);
    assert_eq!(all_inbound.items.len(), 1);
    assert_eq!(all_inbound.total_pages, 2);
}

#[tokio::test]
async fn events_require_audit_read_permission() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "PERM").await;
    seed_pending_inbound(&app, &login.body.access_token, item_id).await;

    let staff_token =
        seed_user_with_permissions_and_login(&app, "event-staff", &["stock.item.read"]).await;
    let forbidden = authorized_empty_request(&app, "GET", "/api/events", &staff_token).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

async fn seed_item(app: &crate::test_support::TestApp, access_token: &str, suffix: &str) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: format!("Event Bottle {suffix}"),
            sku: format!("EVT-{suffix}"),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(app, access_token).await,
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

async fn seed_pending_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
) -> InboundResponse {
    let location_id = bootstrap_location_id(app).await;
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "Event Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity: 4.0,
                unit_price: 2.5,
                location_id,
                batch_no: Some(format!("EVT-{item_id}")),
                expires_at: Some("2027-01-01".to_owned()),
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    json_body(created).await
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
