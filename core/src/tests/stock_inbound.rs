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
        ItemMutationResponse,
    },
    test_support::{error_code, json_body, login_request, seed_stock_location, seeded_app},
};

#[tokio::test]
async fn inbound_create_stays_pending_until_approval_writes_inventory() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_inbound_item(&app, &login.body.access_token).await;
    let location_id = seed_stock_location(&app, "A-01").await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(item_id, location_id),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(created).await;
    assert_eq!(
        serde_json::to_value(order.status).expect("status should encode"),
        "pending"
    );
    assert_eq!(
        order.submission_mode,
        Some(crate::stock::controller::InboundSubmissionMode::PendingApproval)
    );
    assert_eq!(order.items.len(), 1);
    assert_eq!(table_count(&app, "stock_batches").await, 0);
    assert_eq!(table_count(&app, "stock_movements").await, 0);

    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
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
    assert_eq!(audit_count_for_entity(&app, "inbound").await, 2);

    let approved_list = authorized_empty_request(
        &app,
        "GET",
        "/api/inbound?status=approved",
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved_list.status(), StatusCode::OK);
    let approved_list: serde_json::Value = json_body(approved_list).await;
    assert_eq!(approved_list["total"], 1);
    assert_eq!(approved_list["items"][0]["id"], order.id);

    let invalid_status = authorized_empty_request(
        &app,
        "GET",
        "/api/inbound?status=unknown",
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid_status.status(), StatusCode::BAD_REQUEST);

    let approve_again = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_again.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(approve_again).await, "order_not_pending");
}

#[tokio::test]
async fn inbound_direct_submission_approves_atomically_and_reports_mode() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_inbound_item(&app, &login.body.access_token).await;
    let location_id = seed_stock_location(&app, "DIRECT-01").await;
    let mut request = inbound_request(item_id, location_id);
    request.submission_mode = crate::stock::controller::InboundSubmissionMode::Direct;

    let response = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &request,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(response).await;
    assert_eq!(
        serde_json::to_value(order.status).expect("status should encode"),
        "approved"
    );
    assert_eq!(
        order.submission_mode,
        Some(crate::stock::controller::InboundSubmissionMode::Direct)
    );
    assert_eq!(order.approved_by_user_id, order.created_by_user_id);
    assert!(order.approved_at.is_some());
    assert_eq!(table_count(&app, "stock_batches").await, 1);
    assert_eq!(table_count(&app, "stock_movements").await, 1);
    assert_eq!(audit_count_for_entity(&app, "inbound").await, 2);
}

#[tokio::test]
async fn inbound_reject_prevents_later_approval() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_inbound_item(&app, &login.body.access_token).await;
    let location_id = seed_stock_location(&app, "A-01").await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(item_id, location_id),
    )
    .await;
    let order: InboundResponse = json_body(created).await;

    let rejected = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/reject", order.id),
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
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approve_rejected.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn inbound_approval_rejects_location_removed_after_order_creation() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_inbound_item(&app, &login.body.access_token).await;
    let location_id = seed_stock_location(&app, "REMOVED-LOC").await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(item_id, location_id),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(created).await;

    let removed = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/locations/{location_id}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);

    let approval = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approval.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(approval).await, "location_not_found");
    assert_eq!(table_count(&app, "stock_batches").await, 0);
}

#[tokio::test]
async fn inbound_permissions_are_enforced() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_id = seed_inbound_item(&app, &login.body.access_token).await;
    let location_id = seed_stock_location(&app, "A-01").await;

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "inbound-viewer", &["stock.inbound.read"]).await;
    let forbidden_create = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &viewer_token,
        &inbound_request(item_id, location_id),
    )
    .await;
    assert_eq!(forbidden_create.status(), StatusCode::FORBIDDEN);

    let staff_token =
        seed_user_with_permissions_and_login(&app, "inbound-staff", &["stock.inbound.create"])
            .await;
    let forbidden_item_templates =
        authorized_empty_request(&app, "GET", "/api/item-attribute-templates", &staff_token).await;
    assert_eq!(forbidden_item_templates.status(), StatusCode::FORBIDDEN);
    let created_by_staff = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &staff_token,
        &inbound_request(item_id, location_id),
    )
    .await;
    assert_eq!(created_by_staff.status(), StatusCode::CREATED);
    let staff_order: InboundResponse = json_body(created_by_staff).await;
    let forbidden_approve = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", staff_order.id),
        &staff_token,
    )
    .await;
    assert_eq!(forbidden_approve.status(), StatusCode::FORBIDDEN);

    let mut direct_request = inbound_request(item_id, location_id);
    direct_request.submission_mode = crate::stock::controller::InboundSubmissionMode::Direct;
    let forbidden_direct =
        authorized_json_request(&app, "POST", "/api/inbound", &staff_token, &direct_request).await;
    assert_eq!(forbidden_direct.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        error_code(forbidden_direct).await,
        "inbound_direct_approval_forbidden"
    );

    let listed = authorized_empty_request(&app, "GET", "/api/inbound", &viewer_token).await;
    assert_eq!(listed.status(), StatusCode::OK);
}

#[tokio::test]
async fn inbound_search_and_filter_values_use_history_scope() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let primary_item_id = seed_named_item(
        &app,
        &login.body.access_token,
        "InboundSearchBottle",
        "INB-SEARCH-001",
    )
    .await;
    let secondary_item_id = seed_named_item(
        &app,
        &login.body.access_token,
        "InboundSearchCap",
        "INB-SEARCH-002",
    )
    .await;
    let location_l01 = seed_stock_location(&app, "L-01").await;
    let location_l02 = seed_stock_location(&app, "L-02").await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "SpecialSupplier".to_owned(),
            notes: Some("RareNoteNeedle".to_owned()),
            items: vec![
                InboundItemRequest {
                    item_id: primary_item_id,
                    quantity: 10.0,
                    unit_price: 2.5,
                    location_id: location_l01,
                    batch_no: Some("HIST-001".to_owned()),
                    expires_at: Some("2029-01-01".to_owned()),
                },
                InboundItemRequest {
                    item_id: secondary_item_id,
                    quantity: 5.0,
                    unit_price: 3.5,
                    location_id: location_l02,
                    batch_no: Some("HIST-002".to_owned()),
                    expires_at: Some("2029-02-01".to_owned()),
                },
            ],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let order: InboundResponse = json_body(created).await;

    let approved = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
    zero_order_inventory(&app, order.id).await;

    assert_inbound_search_total(&app, &login.body.access_token, "SpecialSupplier", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "Special", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "RareNoteNeedle", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "InboundSearchBottle", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "HIST-001", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "L-01", 1).await;

    let empty_search = authorized_empty_request(
        &app,
        "GET",
        "/api/inbound?search=",
        &login.body.access_token,
    )
    .await;
    assert_eq!(empty_search.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(empty_search).await, "invalid_request");

    let missing_token = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/inbound/filter-values")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let filter_values = authorized_empty_request(
        &app,
        "GET",
        "/api/inbound/filter-values",
        &login.body.access_token,
    )
    .await;
    assert_eq!(filter_values.status(), StatusCode::OK);
    let filter_values: serde_json::Value = json_body(filter_values).await;
    assert_eq!(
        filter_value_count(&filter_values, "base:source", "SpecialSupplier"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:status", "approved"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:item", "InboundSearchBottle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:location", "L-01"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:batch_no", "HIST-001"),
        Some(1)
    );
}

async fn seed_named_item(
    app: &crate::test_support::TestApp,
    access_token: &str,
    name: &str,
    sku: &str,
) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: name.to_owned(),
            sku: sku.to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(app, access_token).await,
            unit: "pcs".to_owned(),
            description: Some("inbound search fixture".to_owned()),
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

async fn assert_inbound_search_total(
    app: &crate::test_support::TestApp,
    access_token: &str,
    search: &str,
    expected_total: u64,
) {
    let response = authorized_empty_request(
        app,
        "GET",
        &format!("/api/inbound?page=1&page_size=20&search={search}"),
        access_token,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = json_body(response).await;
    assert_eq!(payload["total"], expected_total);
}

async fn zero_order_inventory(app: &crate::test_support::TestApp, order_id: i64) {
    app.state
        .database()
        .execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            UPDATE stock_batches
            SET remaining_quantity = 0
            WHERE inbound_order_item_id IN (
                SELECT id FROM stock_inbound_order_items WHERE order_id = ?
            )
            "#,
            [order_id.into()],
        ))
        .await
        .expect("inventory update should succeed");
}

fn filter_value_count(payload: &serde_json::Value, key: &str, value: &str) -> Option<u64> {
    payload["fields"]
        .as_array()?
        .iter()
        .find(|field| field.get("key").and_then(serde_json::Value::as_str) == Some(key))?
        .get("values")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("value").and_then(serde_json::Value::as_str) == Some(value))?
        .get("count")?
        .as_u64()
}

async fn seed_inbound_item(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    seed_named_item(app, access_token, "Inbound Bottle", "INB-FLOW-001").await
}

fn inbound_request(item_id: i64, location_id: i64) -> InboundCreateRequest {
    InboundCreateRequest {
        submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
        source: "Supplier A".to_owned(),
        notes: Some("first inbound".to_owned()),
        items: vec![InboundItemRequest {
            item_id,
            quantity: 10.0,
            unit_price: 2.5,
            location_id,
            batch_no: Some("BATCH-001".to_owned()),
            expires_at: Some("2027-01-01".to_owned()),
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

async fn table_count(app: &crate::test_support::TestApp, table: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) AS count FROM {table}"),
        ))
        .await
        .expect("count query should succeed")
        .expect("count row should exist");

    row.try_get("", "count").expect("count should decode")
}

async fn audit_count_for_entity(app: &crate::test_support::TestApp, entity_type: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM audit_events WHERE entity_type = ?",
            [entity_type.into()],
        ))
        .await
        .expect("audit count query should succeed")
        .expect("audit count row should exist");

    row.try_get("", "count").expect("audit count should decode")
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
