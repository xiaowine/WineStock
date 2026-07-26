//! stock 模块出库单接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest,
        ItemMutationResponse, OutboundCreateRequest, OutboundItemRequest, OutboundResponse,
    },
    test_support::{
        bootstrap_location_id, error_code, json_body, login_request, seed_stock_location,
        seeded_app,
    },
};

#[tokio::test]
async fn outbound_approval_uses_fifo_and_writes_movements() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "FIFO").await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        5.0,
        "FIFO-A",
        "2026-01-01",
    )
    .await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        8.0,
        "FIFO-B",
        "2027-01-01",
    )
    .await;
    assert_eq!(
        table_count(&app, "stock_movements", "movement_type = 'inbound'").await,
        2
    );

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 7.0, None, None),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: OutboundResponse = json_body(created).await;
    assert_eq!(
        serde_json::to_value(order.status).expect("status should encode"),
        "pending"
    );

    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
    let approved: OutboundResponse = json_body(approved).await;
    assert_eq!(
        serde_json::to_value(approved.status).expect("status should encode"),
        "approved"
    );
    assert_eq!(batch_remaining(&app, "FIFO-A").await, 0.0);
    assert_eq!(batch_remaining(&app, "FIFO-B").await, 6.0);
    assert_eq!(
        table_count(&app, "stock_movements", "movement_type = 'outbound'").await,
        2
    );
    assert_eq!(
        table_count(&app, "audit_events", "entity_type = 'outbound'").await,
        2
    );

    let approve_again = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_again.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(approve_again).await, "order_not_pending");
}

#[tokio::test]
async fn outbound_can_deduct_a_specified_batch() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "SPEC").await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        4.0,
        "SPEC-A",
        "2026-01-01",
    )
    .await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        6.0,
        "SPEC-B",
        "2027-01-01",
    )
    .await;
    let target_batch_id = batch_id(&app, "SPEC-B").await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 3.0, Some(target_batch_id), None),
    )
    .await;
    let order: OutboundResponse = json_body(created).await;
    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
    assert_eq!(batch_remaining(&app, "SPEC-A").await, 4.0);
    assert_eq!(batch_remaining(&app, "SPEC-B").await, 3.0);
}

#[tokio::test]
async fn outbound_shortage_rolls_back_inventory_changes() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "SHORT").await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        5.0,
        "SHORT-A",
        "2026-01-01",
    )
    .await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 6.0, None, None),
    )
    .await;
    let order: OutboundResponse = json_body(created).await;
    let failed = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(failed.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(failed).await, "insufficient_stock");
    assert_eq!(batch_remaining(&app, "SHORT-A").await, 5.0);
    assert_eq!(
        table_count(&app, "stock_movements", "movement_type = 'outbound'").await,
        0
    );

    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/outbound/{}", order.id),
        &login.body.access_token,
    )
    .await;
    let detail: OutboundResponse = json_body(detail).await;
    assert_eq!(
        serde_json::to_value(detail.status).expect("status should encode"),
        "pending"
    );
}

#[tokio::test]
async fn outbound_reject_and_permissions_follow_business_rules() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_item(&app, &login.body.access_token, "PERM").await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        5.0,
        "PERM-A",
        "2026-01-01",
    )
    .await;

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "outbound-viewer", &["stock.outbound.read"])
            .await;
    let forbidden_create = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &viewer_token,
        &outbound_request(item_id, 1.0, None, None),
    )
    .await;
    assert_eq!(forbidden_create.status(), StatusCode::FORBIDDEN);

    let staff_token =
        seed_user_with_permissions_and_login(&app, "outbound-staff", &["stock.outbound.create"])
            .await;
    let staff_options =
        authorized_empty_request(&app, "GET", "/api/items/options", &staff_token).await;
    assert_eq!(staff_options.status(), StatusCode::OK);
    let staff_batches = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}/batches"),
        &staff_token,
    )
    .await;
    assert_eq!(staff_batches.status(), StatusCode::OK);
    let staff_created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &staff_token,
        &outbound_request(item_id, 1.0, None, None),
    )
    .await;
    assert_eq!(staff_created.status(), StatusCode::CREATED);
    let staff_order: OutboundResponse = json_body(staff_created).await;
    let forbidden_approve = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", staff_order.id),
        &staff_token,
    )
    .await;
    assert_eq!(forbidden_approve.status(), StatusCode::FORBIDDEN);

    let admin_created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &outbound_request(item_id, 1.0, None, None),
    )
    .await;
    let admin_order: OutboundResponse = json_body(admin_created).await;
    let rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/reject", admin_order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::OK);
    assert_eq!(batch_remaining(&app, "PERM-A").await, 5.0);

    let approve_rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", admin_order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_rejected.status(), StatusCode::CONFLICT);

    let listed = authorized_empty_request(&app, "GET", "/api/outbound", &viewer_token).await;
    assert_eq!(listed.status(), StatusCode::OK);
}

#[tokio::test]
async fn outbound_search_uses_history_scope() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_search_item(&app, &login.body.access_token).await;
    let outbound_location_id = seed_stock_location(&app, "OUT-L-01").await;
    seed_approved_inbound_at_location(
        &app,
        &login.body.access_token,
        item_id,
        10.0,
        "OUT-HIST-001",
        "2029-01-01",
        Some(outbound_location_id),
    )
    .await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/outbound",
        &login.body.access_token,
        &OutboundCreateRequest {
            destination: "SpecialCustomer".to_owned(),
            notes: Some("RareOutboundNote".to_owned()),
            items: vec![OutboundItemRequest {
                item_id,
                quantity: 3.0,
                batch_id: None,
                location_id: Some(outbound_location_id),
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: OutboundResponse = json_body(created).await;
    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/outbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);

    assert_outbound_search_total(&app, &login.body.access_token, "SpecialCustomer", 1).await;
    assert_outbound_search_total(&app, &login.body.access_token, "Special", 1).await;
    assert_outbound_search_total(&app, &login.body.access_token, "RareOutboundNote", 1).await;
    assert_outbound_search_total(&app, &login.body.access_token, "OutboundSearchBottle", 1).await;
    assert_outbound_search_total(&app, &login.body.access_token, "OUT-L-01", 1).await;
    assert_outbound_search_total(&app, &login.body.access_token, "OUT-HIST-001", 1).await;

    let filter_values = authorized_empty_request(
        &app,
        "GET",
        "/api/outbound/filter-values",
        &login.body.access_token,
    )
    .await;
    assert_eq!(filter_values.status(), StatusCode::OK);
    let filter_values: serde_json::Value = json_body(filter_values).await;
    assert_eq!(
        filter_value_count(&filter_values, "base:destination", "SpecialCustomer"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:status", "approved"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:item", "OutboundSearchBottle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:location", "OUT-L-01"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:batch_no", "OUT-HIST-001"),
        Some(1)
    );

    let empty_search = authorized_empty_request(
        &app,
        "GET",
        "/api/outbound?search=",
        &login.body.access_token,
    )
    .await;
    assert_eq!(empty_search.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(empty_search).await, "invalid_request");
}

async fn seed_item(app: &crate::test_support::TestApp, access_token: &str, suffix: &str) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: format!("Outbound Bottle {suffix}"),
            sku: format!("OUT-{suffix}"),
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
    let item: serde_json::Value = json_body(item).await;

    item["id"].as_i64().expect("item id should exist")
}

async fn seed_search_item(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: "OutboundSearchBottle".to_owned(),
            sku: "OUT-SEARCH-001".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(app, access_token).await,
            unit: "pcs".to_owned(),
            description: Some("outbound search fixture".to_owned()),
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);
    let item: ItemMutationResponse = json_body(item).await;

    item.id
}

async fn seed_approved_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    quantity: f64,
    batch_no: &str,
    expires_at: &str,
) {
    seed_approved_inbound_at_location(
        app,
        access_token,
        item_id,
        quantity,
        batch_no,
        expires_at,
        None,
    )
    .await;
}

async fn seed_approved_inbound_at_location(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    quantity: f64,
    batch_no: &str,
    expires_at: &str,
    location_id: Option<i64>,
) {
    let location_id = match location_id {
        Some(location_id) => location_id,
        None => bootstrap_location_id(app).await,
    };
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity,
                unit_price: 2.5,
                location_id,
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some(expires_at.to_owned()),
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

async fn assert_outbound_search_total(
    app: &crate::test_support::TestApp,
    access_token: &str,
    search: &str,
    expected_total: u64,
) {
    let response = authorized_empty_request(
        app,
        "GET",
        &format!("/api/outbound?page=1&page_size=20&search={search}"),
        access_token,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = json_body(response).await;
    assert_eq!(body["total"].as_u64(), Some(expected_total));
    assert_eq!(
        body["items"]
            .as_array()
            .expect("items should be array")
            .len(),
        expected_total as usize
    );
}

fn filter_value_count(payload: &serde_json::Value, key: &str, value: &str) -> Option<u64> {
    payload["fields"]
        .as_array()?
        .iter()
        .find(|field| field["key"] == key)?["values"]
        .as_array()?
        .iter()
        .find(|candidate| candidate["value"] == value)?["count"]
        .as_u64()
}

fn outbound_request(
    item_id: i64,
    quantity: f64,
    batch_id: Option<i64>,
    location_id: Option<i64>,
) -> OutboundCreateRequest {
    OutboundCreateRequest {
        destination: "Cellar".to_owned(),
        notes: Some("test outbound".to_owned()),
        items: vec![OutboundItemRequest {
            item_id,
            quantity,
            batch_id,
            location_id,
        }],
    }
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

async fn batch_id(app: &crate::test_support::TestApp, batch_no: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id FROM stock_batches WHERE batch_no = ?",
            [batch_no.into()],
        ))
        .await
        .expect("batch query should succeed")
        .expect("batch row should exist");

    row.try_get("", "id").expect("batch id should decode")
}

async fn batch_remaining(app: &crate::test_support::TestApp, batch_no: &str) -> f64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT remaining_quantity FROM stock_batches WHERE batch_no = ?",
            [batch_no.into()],
        ))
        .await
        .expect("batch query should succeed")
        .expect("batch row should exist");

    row.try_get("", "remaining_quantity")
        .expect("remaining quantity should decode")
}

async fn table_count(app: &crate::test_support::TestApp, table: &str, filter: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) AS count FROM {table} WHERE {filter}"),
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
