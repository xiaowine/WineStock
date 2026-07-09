//! stock 模块物品接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest,
        ItemDetailResponse, ItemResponse, ItemUpdateRequest, TemplateCreateRequest,
        TemplateFieldDef, TemplateFieldType, TemplateResponse,
    },
    test_support::{json_body, json_request, login_request, seeded_app, text_body},
};

#[tokio::test]
async fn item_crud_uses_permissions_and_soft_delete() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    assert_eq!(login.status, StatusCode::OK);

    let missing_token = app
        .router
        .clone()
        .oneshot(json_request("GET", "/api/items", &serde_json::json!({})))
        .await
        .expect("request should complete");
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "  Cabernet Cork  ".to_owned(),
            sku: " CORK-001 ".to_owned(),
            category_id: None,
            unit: "pcs".to_owned(),
            description: Some("Bottle closure".to_owned()),
            default_price: Some(1.25),
            reorder_point: Some(10.0),
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let item: ItemResponse = json_body(created).await;
    assert_eq!(item.name, "Cabernet Cork");
    assert_eq!(item.sku, "CORK-001");

    let duplicate = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "Duplicate".to_owned(),
            sku: "CORK-001".to_owned(),
            category_id: None,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(duplicate).await, "sku_taken");

    let listed = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=cork",
        &login.body.access_token,
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let list: serde_json::Value = json_body(listed).await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["total_pages"], 1);
    assert_eq!(list["items"][0]["sku"], "CORK-001");

    let updated = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/items/{}", item.id),
        &login.body.access_token,
        &ItemUpdateRequest {
            name: Some("Reserve Cork".to_owned()),
            sku: Some("CORK-002".to_owned()),
            category_id: None,
            unit: None,
            description: Some("Updated closure".to_owned()),
            default_price: Some(1.50),
            reorder_point: Some(12.0),
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: ItemResponse = json_body(updated).await;
    assert_eq!(updated.name, "Reserve Cork");
    assert_eq!(updated.sku, "CORK-002");

    let deleted = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/items/{}", item.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let missing = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{}", item.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn item_detail_returns_current_inventory_summary() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let template_id = seed_item_search_template(&app, &login.body.access_token).await;
    let item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Detail Sensor",
        "DETAIL-001",
    )
    .await;

    let empty_detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(empty_detail.status(), StatusCode::OK);
    let empty_detail: ItemDetailResponse = json_body(empty_detail).await;
    assert_eq!(empty_detail.current_quantity, 0.0);
    assert_eq!(empty_detail.inventory_value, 0.0);
    assert!(empty_detail.locations.is_empty());
    assert!(empty_detail.batches.is_empty());

    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "DetailNeedle",
        "PrivateNeedle",
        "A-01",
        "DETAIL-BATCH-001",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "DetailNeedle",
        "PrivateNeedle",
        "B-02",
        "DETAIL-BATCH-002",
    )
    .await;

    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail: ItemDetailResponse = json_body(detail).await;
    assert_eq!(detail.id, item_id);
    assert_eq!(detail.current_quantity, 20.0);
    assert_eq!(detail.inventory_value, 50.0);
    assert_eq!(detail.locations.len(), 2);
    assert_eq!(detail.locations[0].location.as_deref(), Some("A-01"));
    assert_eq!(detail.locations[0].quantity, 10.0);
    assert_eq!(detail.locations[0].value, 25.0);
    assert_eq!(detail.locations[0].batch_count, 1);
    assert_eq!(detail.locations[1].location.as_deref(), Some("B-02"));
    assert_eq!(detail.batches.len(), 2);
    assert_eq!(detail.batches[0].batch_no, "DETAIL-BATCH-001");
    assert_eq!(detail.batches[0].remaining_quantity, 10.0);
    assert_eq!(detail.batches[0].unit_cost, 2.5);
    assert_eq!(detail.batches[0].value, 25.0);
    assert_eq!(detail.batches[1].batch_no, "DETAIL-BATCH-002");
}

#[tokio::test]
async fn item_validation_and_authorization_fail_before_write() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let invalid = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "Bad".to_owned(),
            sku: "BAD-001".to_owned(),
            category_id: None,
            unit: "pcs".to_owned(),
            description: None,
            default_price: Some(-1.0),
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(text_body(invalid).await, "invalid_request");

    let forbidden_token =
        seed_user_with_permissions_and_login(&app, "viewer", &["stock.read"]).await;
    let forbidden = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &forbidden_token,
        &ItemCreateRequest {
            name: "Viewer Item".to_owned(),
            sku: "VIEW-001".to_owned(),
            category_id: None,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn item_search_and_filter_values_use_current_inventory_template_values() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let template_id = seed_item_search_template(&app, &login.body.access_token).await;
    let item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Searchable Sensor",
        "SEARCH-001",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "CurrentNeedle",
        "PrivateNeedle",
        "A-01",
        "CUR-001",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "CurrentNeedle",
        "PrivateNeedle",
        "A-02",
        "CUR-002",
    )
    .await;

    let historical_item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Historical Sensor",
        "HIST-001",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        historical_item_id,
        "GoneNeedle",
        "HiddenGoneNeedle",
        "Z-99",
        "GONE-001",
    )
    .await;
    zero_item_inventory(&app, historical_item_id).await;

    let by_template_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=CurrentNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_template_value.status(), StatusCode::OK);
    let by_template_value: serde_json::Value = json_body(by_template_value).await;
    assert_eq!(by_template_value["total"], 1);
    assert_eq!(by_template_value["items"][0]["id"], item_id);

    let by_non_searchable_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=PrivateNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_non_searchable_value.status(), StatusCode::OK);
    let by_non_searchable_value: serde_json::Value = json_body(by_non_searchable_value).await;
    assert_eq!(by_non_searchable_value["total"], 1);

    let by_template_name = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=SearchFilterTemplate",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_template_name.status(), StatusCode::OK);
    let by_template_name: serde_json::Value = json_body(by_template_name).await;
    assert_eq!(by_template_name["total"], 2);

    let by_exhausted_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=GoneNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_exhausted_value.status(), StatusCode::OK);
    let by_exhausted_value: serde_json::Value = json_body(by_exhausted_value).await;
    assert_eq!(by_exhausted_value["total"], 0);

    let empty_search =
        authorized_empty_request(&app, "GET", "/api/items?search=", &login.body.access_token).await;
    assert_eq!(empty_search.status(), StatusCode::BAD_REQUEST);
    assert_eq!(text_body(empty_search).await, "invalid_request");

    let missing_token = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/items/filter-values")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let filter_values = authorized_empty_request(
        &app,
        "GET",
        "/api/items/filter-values",
        &login.body.access_token,
    )
    .await;
    assert_eq!(filter_values.status(), StatusCode::OK);
    let filter_values: serde_json::Value = json_body(filter_values).await;
    assert_eq!(
        filter_value_count(&filter_values, "template:brand", "CurrentNeedle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "template:brand", "GoneNeedle"),
        None
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:unit", "pcs"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:location", "A-01"),
        Some(1)
    );
    assert!(!has_filter_field(&filter_values, "template:internal_note"));
}

async fn seed_item_search_template(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/templates",
        access_token,
        &TemplateCreateRequest {
            name: "SearchFilterTemplate".to_owned(),
            description: Some("search metadata template".to_owned()),
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
                    field_name: "internal_note".to_owned(),
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let template: TemplateResponse = json_body(response).await;

    template.id
}

async fn seed_item(
    app: &crate::test_support::TestApp,
    access_token: &str,
    template_id: i64,
    name: &str,
    sku: &str,
) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: name.to_owned(),
            sku: sku.to_owned(),
            category_id: Some(template_id),
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let item: ItemResponse = json_body(response).await;

    item.id
}

async fn create_and_approve_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    brand: &str,
    internal_note: &str,
    location: &str,
    batch_no: &str,
) -> InboundResponse {
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            source: "Search Supplier".to_owned(),
            notes: Some("search fixture".to_owned()),
            items: vec![InboundItemRequest {
                item_id,
                quantity: 10.0,
                unit_price: 2.5,
                location: Some(location.to_owned()),
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some("2028-01-01".to_owned()),
                ext_attributes: Some(serde_json::json!({
                    "brand": brand,
                    "internal_note": internal_note
                })),
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
    json_body(approved).await
}

async fn zero_item_inventory(app: &crate::test_support::TestApp, item_id: i64) {
    app.state
        .database()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE stock_batches SET remaining_quantity = 0 WHERE item_id = ?",
            [item_id.into()],
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
