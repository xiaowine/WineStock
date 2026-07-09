//! stock 模块入库单接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest, ItemResponse,
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
            Some(serde_json::json!({
                "brand": "Acme",
                "abv": 13.5,
                "datasheet": "https://example.com/spec"
            })),
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

    let approve_again = authorized_empty_request(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
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
        &format!("/api/stock-approvals/inbound/{}/approve", order.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid_approval.status(), StatusCode::BAD_REQUEST);
    assert_eq!(table_count(&app, "stock_batches").await, 0);

    let invalid_url = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &inbound_request(
            item_id,
            Some(serde_json::json!({"brand": "Acme", "datasheet": "example.com/spec"})),
        ),
    )
    .await;
    assert_eq!(invalid_url.status(), StatusCode::CREATED);
    let invalid_url_order: InboundResponse = json_body(invalid_url).await;
    let invalid_url_approval = authorized_empty_request(
        &app,
        "POST",
        &format!(
            "/api/stock-approvals/inbound/{}/approve",
            invalid_url_order.id
        ),
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid_url_approval.status(), StatusCode::BAD_REQUEST);

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "inbound-viewer", &["stock.inbound.read"]).await;
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

    let staff_token =
        seed_user_with_permissions_and_login(&app, "inbound-staff", &["stock.inbound.create"])
            .await;
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
        &format!("/api/stock-approvals/inbound/{}/approve", staff_order.id),
        &staff_token,
    )
    .await;
    assert_eq!(forbidden_approve.status(), StatusCode::FORBIDDEN);

    let listed = authorized_empty_request(&app, "GET", "/api/inbound", &viewer_token).await;
    assert_eq!(listed.status(), StatusCode::OK);
}

#[tokio::test]
async fn inbound_search_and_filter_values_use_history_scope() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let (primary_item_id, secondary_item_id) =
        seed_inbound_search_items(&app, &login.body.access_token).await;

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/inbound",
        &login.body.access_token,
        &InboundCreateRequest {
            source: "SpecialSupplier".to_owned(),
            notes: Some("RareNoteNeedle".to_owned()),
            items: vec![
                InboundItemRequest {
                    item_id: primary_item_id,
                    quantity: 10.0,
                    unit_price: 2.5,
                    location: Some("L-01".to_owned()),
                    batch_no: Some("HIST-001".to_owned()),
                    expires_at: Some("2029-01-01".to_owned()),
                    ext_attributes: Some(serde_json::json!({
                        "brand": "HistoryNeedle",
                        "hidden_note": "HiddenHistoryNeedle"
                    })),
                },
                InboundItemRequest {
                    item_id: secondary_item_id,
                    quantity: 5.0,
                    unit_price: 3.5,
                    location: Some("L-02".to_owned()),
                    batch_no: Some("HIST-002".to_owned()),
                    expires_at: Some("2029-02-01".to_owned()),
                    ext_attributes: Some(serde_json::json!({
                        "brand": "HistoryNeedle",
                        "hidden_note": "HiddenHistoryNeedle"
                    })),
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
    assert_inbound_search_total(&app, &login.body.access_token, "HistoryNeedle", 1).await;
    assert_inbound_search_total(&app, &login.body.access_token, "HiddenHistoryNeedle", 1).await;

    let empty_search = authorized_empty_request(
        &app,
        "GET",
        "/api/inbound?search=",
        &login.body.access_token,
    )
    .await;
    assert_eq!(empty_search.status(), StatusCode::BAD_REQUEST);
    assert_eq!(text_body(empty_search).await, "invalid_request");

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
        filter_value_count(&filter_values, "template:brand", "HistoryNeedle"),
        Some(1)
    );
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
    assert!(!has_filter_field(&filter_values, "template:hidden_note"));
}

async fn seed_inbound_search_items(
    app: &crate::test_support::TestApp,
    access_token: &str,
) -> (i64, i64) {
    let template = authorized_json_request(
        app,
        "POST",
        "/api/templates",
        access_token,
        &TemplateCreateRequest {
            name: "InboundSearchTemplate".to_owned(),
            description: None,
            fields: vec![
                TemplateFieldDef {
                    field_name: "brand".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(true),
                    options: None,
                    default_value: None,
                },
                TemplateFieldDef {
                    field_name: "hidden_note".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(false),
                    options: None,
                    default_value: None,
                },
            ],
        },
    )
    .await;
    assert_eq!(template.status(), StatusCode::CREATED);
    let template: TemplateResponse = json_body(template).await;

    let primary = seed_named_item(
        app,
        access_token,
        template.id,
        "InboundSearchBottle",
        "INB-SEARCH-001",
    )
    .await;
    let secondary = seed_named_item(
        app,
        access_token,
        template.id,
        "InboundSearchCap",
        "INB-SEARCH-002",
    )
    .await;

    (primary, secondary)
}

async fn seed_named_item(
    app: &crate::test_support::TestApp,
    access_token: &str,
    template_id: i64,
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
            category_id: Some(template_id),
            unit: "pcs".to_owned(),
            description: Some("inbound search fixture".to_owned()),
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);
    let item: ItemResponse = json_body(item).await;

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
        .execute(Statement::from_sql_and_values(
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

fn has_filter_field(payload: &serde_json::Value, key: &str) -> bool {
    payload["fields"].as_array().is_some_and(|fields| {
        fields
            .iter()
            .any(|field| field.get("key").and_then(serde_json::Value::as_str) == Some(key))
    })
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
                TemplateFieldDef {
                    field_name: "datasheet".to_owned(),
                    field_type: TemplateFieldType::Url,
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
        .query_one(Statement::from_string(
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
        .query_one(Statement::from_sql_and_values(
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
