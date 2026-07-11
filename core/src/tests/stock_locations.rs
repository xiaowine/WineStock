//! stock 模块库位接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::{
        controller::{
            InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest,
            ItemDetailResponse, LocationCreateRequest, LocationGroupCreateRequest,
            LocationGroupResponse, LocationGroupTreeNode, LocationGroupUpdateRequest,
            LocationResponse, LocationTransferCreateRequest, LocationTransferResponse,
        },
        service::PaginatedResponse,
    },
    test_support::{error_code, json_body, login_request, seeded_app},
};

#[tokio::test]
async fn default_location_and_group_tree_follow_hierarchy_rules() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let tree = authorized_empty_request(
        &app,
        "GET",
        "/api/location-groups/tree",
        &login.body.access_token,
    )
    .await;
    assert_eq!(tree.status(), StatusCode::OK);
    let tree: Vec<LocationGroupTreeNode> = json_body(tree).await;
    let default_group = tree
        .iter()
        .find(|group| group.name == "默认库区")
        .expect("default group should exist");
    assert!(default_group.parent_id.is_none());
    assert!(default_group
        .locations
        .iter()
        .any(|location| location.code == "DEFAULT" && location.name == "默认库位"));

    let root = create_group(&app, &login.body.access_token, None, "主仓").await;
    let child = create_group(&app, &login.body.access_token, Some(root.id), "A区").await;
    let duplicate_child = authorized_json_request(
        &app,
        "POST",
        "/api/location-groups",
        &login.body.access_token,
        &LocationGroupCreateRequest {
            parent_id: Some(root.id),
            name: "A区".to_owned(),
            sort_order: Some(1),
        },
    )
    .await;
    assert_eq!(duplicate_child.status(), StatusCode::CONFLICT);
    assert_eq!(
        error_code(duplicate_child).await,
        "location_group_name_taken"
    );

    let cycle = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/location-groups/{}", root.id),
        &login.body.access_token,
        &LocationGroupUpdateRequest {
            parent_id: Some(child.id),
            name: root.name.clone(),
            sort_order: Some(root.sort_order),
        },
    )
    .await;
    assert_eq!(cycle.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(cycle).await, "location_group_cycle");

    let delete_root = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/location-groups/{}", root.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete_root.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(delete_root).await, "location_group_in_use");
}

#[tokio::test]
async fn locations_can_be_created_moved_and_protected_by_current_stock() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let group_id = default_group_id(&app).await;
    let from_location = create_location(&app, &login.body.access_token, group_id, "A-01").await;
    let to_location = create_location(&app, &login.body.access_token, group_id, "B-02").await;
    let item_id = seed_item(&app, &login.body.access_token).await;
    let inbound = seed_approved_inbound(
        &app,
        &login.body.access_token,
        item_id,
        from_location.id,
        "MOVE-BATCH",
    )
    .await;
    assert_eq!(inbound.items[0].location_id, from_location.id);
    let batch_id = batch_id_by_no(&app, "MOVE-BATCH").await;

    let transfer = authorized_json_request(
        &app,
        "POST",
        "/api/location-transfers",
        &login.body.access_token,
        &LocationTransferCreateRequest {
            batch_id,
            from_location_id: from_location.id,
            to_location_id: to_location.id,
            notes: Some("整批移库".to_owned()),
        },
    )
    .await;
    assert_eq!(transfer.status(), StatusCode::CREATED);
    let transfer: LocationTransferResponse = json_body(transfer).await;
    assert_eq!(transfer.batch_id, batch_id);
    assert_eq!(transfer.quantity, 5.0);

    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}"),
        &login.body.access_token,
    )
    .await;
    let detail: ItemDetailResponse = json_body(detail).await;
    assert_eq!(detail.locations.len(), 1);
    assert_eq!(detail.locations[0].location_id, to_location.id);
    assert_eq!(detail.locations[0].location_code, "B-02");

    let delete_busy_location = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/locations/{}", to_location.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete_busy_location.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(delete_busy_location).await, "location_in_use");

    let delete_empty_location = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/locations/{}", from_location.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete_empty_location.status(), StatusCode::NO_CONTENT);

    let events = authorized_empty_request(
        &app,
        "GET",
        "/api/events?entity_type=location_transfer&action=created",
        &login.body.access_token,
    )
    .await;
    assert_eq!(events.status(), StatusCode::OK);
    let events: PaginatedResponse<crate::stock::controller::EventLogResponse> =
        json_body(events).await;
    assert_eq!(events.total, 1);
    assert_eq!(events.items[0].details["to_location_id"], to_location.id);
}

async fn create_group(
    app: &crate::test_support::TestApp,
    access_token: &str,
    parent_id: Option<i64>,
    name: &str,
) -> LocationGroupResponse {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/location-groups",
        access_token,
        &LocationGroupCreateRequest {
            parent_id,
            name: name.to_owned(),
            sort_order: Some(0),
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn create_location(
    app: &crate::test_support::TestApp,
    access_token: &str,
    group_id: i64,
    code: &str,
) -> LocationResponse {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/locations",
        access_token,
        &LocationCreateRequest {
            group_id,
            code: code.to_owned(),
            name: format!("{code} 库位"),
            sort_order: Some(0),
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body(response).await
}

async fn seed_item(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: "Location Transfer Item".to_owned(),
            sku: "LOC-TRANSFER-001".to_owned(),
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let item: serde_json::Value = json_body(response).await;

    item["id"].as_i64().expect("item id should exist")
}

async fn seed_approved_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    location_id: i64,
    batch_no: &str,
) -> InboundResponse {
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "Location Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity: 5.0,
                unit_price: 2.5,
                location_id,
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some("2029-01-01".to_owned()),
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
    json_body(approved).await
}

async fn default_group_id(app: &crate::test_support::TestApp) -> i64 {
    app.state
        .database()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id FROM stock_location_groups WHERE name = '默认库区'".to_owned(),
        ))
        .await
        .expect("default group query should succeed")
        .expect("default group should exist")
        .try_get("", "id")
        .expect("default group id should decode")
}

async fn batch_id_by_no(app: &crate::test_support::TestApp, batch_no: &str) -> i64 {
    app.state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id FROM stock_batches WHERE batch_no = ?",
            [batch_no.into()],
        ))
        .await
        .expect("batch query should succeed")
        .expect("batch should exist")
        .try_get("", "id")
        .expect("batch id should decode")
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
