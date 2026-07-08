//! stock 模块入库单接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest,
        TemplateCreateRequest, TemplateFieldDef, TemplateFieldType, TemplateResponse,
    },
    test_support::{json_body, login_request, seeded_app, text_body},
};

#[tokio::test]
async fn inbound_create_stays_pending_until_approval_writes_inventory() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_template_bound_item(&app, &login.body.access_token).await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(
            item_id,
            Some(serde_json::json!({"brand": "Acme", "abv": 13.5})),
        ),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(created).await;
    assert_eq!(
        serde_json::to_value(order.status).expect("status should encode"),
        "pending"
    );
    assert_eq!(order.items.len(), 1);
    assert_eq!(table_count(&app, "stock_batches").await, 0);
    assert_eq!(table_count(&app, "stock_movements").await, 0);

    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
    let approved: InboundResponse = json_body(approved).await;
    assert_eq!(
        serde_json::to_value(approved.status).expect("status should encode"),
        "approved"
    );
    assert_eq!(table_count(&app, "stock_batches").await, 1);
    assert_eq!(table_count(&app, "stock_movements").await, 1);
    assert_eq!(table_count(&app, "audit_events").await, 2);

    let approve_again = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_again.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(approve_again).await, "order_not_pending");
}

#[tokio::test]
async fn inbound_reject_prevents_later_approval() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_template_bound_item(&app, &login.body.access_token).await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(
            item_id,
            Some(serde_json::json!({"brand": "Acme", "abv": 13.5})),
        ),
    )
    .await;
    let order: InboundResponse = json_body(created).await;

    let rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/reject", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::OK);
    let rejected: InboundResponse = json_body(rejected).await;
    assert_eq!(
        serde_json::to_value(rejected.status).expect("status should encode"),
        "rejected"
    );
    assert_eq!(table_count(&app, "stock_batches").await, 0);

    let approve_rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_rejected.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn inbound_validates_template_attributes_and_permissions() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_template_bound_item(&app, &login.body.access_token).await;

    let missing_required = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(item_id, Some(serde_json::json!({"abv": 13.5}))),
    )
    .await;
    let order: InboundResponse = json_body(missing_required).await;
    let invalid_approval = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid_approval.status(), StatusCode::BAD_REQUEST);
    assert_eq!(table_count(&app, "stock_batches").await, 0);

    let viewer_token = seed_user_with_role_and_login(&app, "inbound-viewer", "viewer").await;
    let forbidden_create = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &viewer_token,
        &inbound_request(
            item_id,
            Some(serde_json::json!({"brand": "Acme", "abv": 13.5})),
        ),
    )
    .await;
    assert_eq!(forbidden_create.status(), StatusCode::FORBIDDEN);

    let staff_token = seed_user_with_role_and_login(&app, "inbound-staff", "staff").await;
    let created_by_staff = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &staff_token,
        &inbound_request(
            item_id,
            Some(serde_json::json!({"brand": "Acme", "abv": 13.5})),
        ),
    )
    .await;
    assert_eq!(created_by_staff.status(), StatusCode::CREATED);
    let staff_order: InboundResponse = json_body(created_by_staff).await;
    let forbidden_approve = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/inbound/{}/approve", staff_order.id),
        &staff_token,
    )
    .await;
    assert_eq!(forbidden_approve.status(), StatusCode::FORBIDDEN);

    let listed = authorized_empty_request(&app, "GET", "/api/inbound", &viewer_token).await;
    assert_eq!(listed.status(), StatusCode::OK);
}

async fn seed_template_bound_item(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    let template = authorized_json_request(
        app,
        "POST",
        "/api/templates",
        access_token,
        &TemplateCreateRequest {
            name: "Inbound Material".to_owned(),
            description: None,
            fields: vec![
                TemplateFieldDef {
                    field_name: "brand".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(true),
                    searchable: Some(true),
                    options: None,
                    default_value: None,
                },
                TemplateFieldDef {
                    field_name: "abv".to_owned(),
                    field_type: TemplateFieldType::Number,
                    required: Some(false),
                    searchable: Some(false),
                    options: None,
                    default_value: None,
                },
            ],
        },
    )
    .await;
    let template: TemplateResponse = json_body(template).await;
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: "Inbound Bottle".to_owned(),
            sku: format!("INB-{}", template.id),
            category_id: Some(template.id),
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    let item: serde_json::Value = json_body(item).await;

    item["id"].as_i64().expect("item id should exist")
}

fn inbound_request(
    item_id: i64,
    ext_attributes: Option<serde_json::Value>,
) -> InboundCreateRequest {
    InboundCreateRequest {
        source: "Supplier A".to_owned(),
        notes: Some("first inbound".to_owned()),
        items: vec![InboundItemRequest {
            item_id,
            quantity: 10.0,
            unit_price: 2.5,
            location: Some("A-01".to_owned()),
            batch_no: Some("BATCH-001".to_owned()),
            expires_at: Some("2027-01-01".to_owned()),
            ext_attributes,
        }],
    }
}

async fn seed_user_with_role_and_login(
    app: &crate::test_support::TestApp,
    username: &str,
    role_code: &str,
) -> String {
    crate::test_support::seed_plain_user(app.state.database(), username, "password").await;
    let rbac = crate::persistence::repository::RbacRepository::new(app.state.database());
    let role_id = rbac
        .ensure_role(role_code, role_code, None)
        .await
        .expect("role should exist");
    let users = crate::persistence::repository::UserRepository::new(app.state.database());
    let user = users
        .find_by_username(username)
        .await
        .expect("user lookup should succeed")
        .expect("user should exist");
    rbac.assign_role_to_user(user.id, role_id)
        .await
        .expect("role should assign");

    login_request(app, username, "password")
        .await
        .body
        .access_token
}

async fn table_count(app: &crate::test_support::TestApp, table: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) AS count FROM {table}"),
        ))
        .await
        .expect("count query should succeed")
        .expect("count row should exist");

    row.try_get("", "count").expect("count should decode")
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
