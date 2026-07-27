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
            ItemInventoryResponse, LocationCreateRequest, LocationGroupCreateRequest,
            LocationGroupResponse, LocationGroupTreeNode, LocationGroupUpdateRequest,
            LocationResponse, LocationTransferCreateRequest, LocationTransferResponse,
            LocationUpdateRequest,
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
        .find(|group| group.name == "示例库区")
        .expect("default group should exist");
    assert!(default_group.parent_id.is_none());
    assert!(default_group
        .locations
        .iter()
        .any(|location| location.name == "示例库位" && location.notes.is_none()));

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

    let mut parent_id = None;
    let mut depth_chain = Vec::new();
    for depth in 1..=10 {
        let group = create_group(
            &app,
            &login.body.access_token,
            parent_id,
            &format!("深度测试 {depth}"),
        )
        .await;
        parent_id = Some(group.id);
        depth_chain.push(group);
    }
    let too_deep = authorized_json_request(
        &app,
        "POST",
        "/api/location-groups",
        &login.body.access_token,
        &LocationGroupCreateRequest {
            parent_id,
            name: "第十一层".to_owned(),
            sort_order: Some(0),
        },
    )
    .await;
    assert_eq!(too_deep.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(too_deep).await, "location_group_depth_exceeded");

    let movable_root = create_group(&app, &login.body.access_token, None, "待移动分组").await;
    create_group(
        &app,
        &login.body.access_token,
        Some(movable_root.id),
        "待移动子分组",
    )
    .await;
    let move_too_deep = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/location-groups/{}", movable_root.id),
        &login.body.access_token,
        &LocationGroupUpdateRequest {
            parent_id: Some(depth_chain[8].id),
            name: movable_root.name.clone(),
            sort_order: Some(movable_root.sort_order),
        },
    )
    .await;
    assert_eq!(move_too_deep.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_code(move_too_deep).await,
        "location_group_depth_exceeded"
    );

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
    assert_eq!(from_location.notes.as_deref(), Some("测试库位备注"));
    let duplicate_name = authorized_json_request(
        &app,
        "POST",
        "/api/locations",
        &login.body.access_token,
        &LocationCreateRequest {
            group_id,
            name: "A-01".to_owned(),
            notes: None,
            sort_order: Some(1),
        },
    )
    .await;
    assert_eq!(duplicate_name.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(duplicate_name).await, "location_name_taken");
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
        &format!("/api/items/{item_id}/inventory"),
        &login.body.access_token,
    )
    .await;
    let detail: ItemInventoryResponse = json_body(detail).await;
    assert_eq!(detail.locations.len(), 1);
    assert_eq!(detail.locations[0].location_id, to_location.id);
    assert_eq!(detail.locations[0].location_name, "B-02");

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

#[tokio::test]
async fn location_global_default_is_unique_and_cleared_on_delete() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let group_id = default_group_id(&app).await;
    let first = create_location(&app, &login.body.access_token, group_id, "默认候选一").await;
    let second = create_location(&app, &login.body.access_token, group_id, "默认候选二").await;
    assert!(!first.is_default);
    assert!(!second.is_default);

    // 设为默认：响应携带 is_default。
    let set_first = set_location_default(&app, &login.body.access_token, &first, Some(true)).await;
    assert!(set_first.is_default);

    // 换默认：服务层事务自动清除旧默认，全表至多一个。
    let set_second =
        set_location_default(&app, &login.body.access_token, &second, Some(true)).await;
    assert!(set_second.is_default);
    let defaults = list_default_location_ids(&app, &login.body.access_token).await;
    assert_eq!(defaults, vec![second.id]);

    // 请求不带 is_default 时保持现状。
    let keep = set_location_default(&app, &login.body.access_token, &second, None).await;
    assert!(keep.is_default);

    // 取消默认后全表无默认。
    let unset = set_location_default(&app, &login.body.access_token, &second, Some(false)).await;
    assert!(!unset.is_default);
    assert!(list_default_location_ids(&app, &login.body.access_token)
        .await
        .is_empty());

    // 删除默认库位随删清空，不迁移默认。
    set_location_default(&app, &login.body.access_token, &first, Some(true)).await;
    let delete = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/locations/{}", first.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    assert!(list_default_location_ids(&app, &login.body.access_token)
        .await
        .is_empty());
}

async fn set_location_default(
    app: &crate::test_support::TestApp,
    access_token: &str,
    location: &LocationResponse,
    is_default: Option<bool>,
) -> LocationResponse {
    let response = authorized_json_request(
        app,
        "PUT",
        &format!("/api/locations/{}", location.id),
        access_token,
        &LocationUpdateRequest {
            group_id: location.group_id,
            name: location.name.clone(),
            notes: location.notes.clone(),
            sort_order: Some(location.sort_order),
            is_default,
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn list_default_location_ids(
    app: &crate::test_support::TestApp,
    access_token: &str,
) -> Vec<i64> {
    let response = authorized_empty_request(app, "GET", "/api/locations", access_token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let locations: Vec<LocationResponse> = json_body(response).await;
    locations
        .into_iter()
        .filter(|location| location.is_default)
        .map(|location| location.id)
        .collect()
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
    name: &str,
) -> LocationResponse {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/locations",
        access_token,
        &LocationCreateRequest {
            group_id,
            name: name.to_owned(),
            notes: Some("测试库位备注".to_owned()),
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
            "SELECT id FROM stock_location_groups WHERE name = '示例库区'".to_owned(),
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
