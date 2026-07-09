//! stock 模块替代料接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, ItemCreateRequest,
        SubstituteBindRequest, SubstituteDetailResponse, SubstituteItem,
        SubstituteRelationResponse,
    },
    test_support::{json_body, login_request, seeded_app, text_body},
};

#[tokio::test]
async fn substitutes_can_be_bound_listed_and_deleted_with_permissions() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let main_id = seed_item(&app, &login.body.access_token, "MAIN").await;
    let substitute_a = seed_item(&app, &login.body.access_token, "SUB-A").await;
    let substitute_b = seed_item(&app, &login.body.access_token, "SUB-B").await;
    seed_approved_inbound(
        &app,
        &login.body.access_token,
        substitute_a,
        6.0,
        "SUB-A-BATCH",
    )
    .await;

    let viewer_token =
        seed_user_with_permissions_and_login(&app, "sub-viewer", &["stock.substitute.read"]).await;
    let forbidden_bind = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{main_id}/substitutes"),
        &viewer_token,
        &substitute_request(vec![(substitute_a, 1, None)]),
    )
    .await;
    assert_eq!(forbidden_bind.status(), StatusCode::FORBIDDEN);

    let bound = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{main_id}/substitutes"),
        &login.body.access_token,
        &substitute_request(vec![
            (substitute_b, 2, Some("fallback")),
            (substitute_a, 1, Some("preferred")),
        ]),
    )
    .await;
    assert_eq!(bound.status(), StatusCode::OK);
    let bound: Vec<SubstituteDetailResponse> = json_body(bound).await;
    assert_eq!(bound.len(), 2);
    assert_eq!(bound[0].substitute_item_id, substitute_a);
    assert_eq!(bound[0].priority, 1);
    assert_eq!(bound[0].notes.as_deref(), Some("preferred"));
    assert_close(bound[0].quantity, 6.0);
    assert_eq!(bound[1].substitute_item_id, substitute_b);

    let listed = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{main_id}/substitutes"),
        &viewer_token,
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed: Vec<SubstituteDetailResponse> = json_body(listed).await;
    assert_eq!(listed.len(), 2);

    let all_relations =
        authorized_empty_request(&app, "GET", "/api/items/substitutes", &viewer_token).await;
    assert_eq!(all_relations.status(), StatusCode::OK);
    let all_relations: Vec<SubstituteRelationResponse> = json_body(all_relations).await;
    assert_eq!(all_relations.len(), 2);
    assert_eq!(all_relations[0].item_id, main_id);
    assert_eq!(all_relations[0].item_name, "Substitute Bottle MAIN");
    assert_eq!(all_relations[0].item_sku, "SUB-MAIN");
    assert_eq!(all_relations[0].substitute_item_id, substitute_a);
    assert_eq!(
        all_relations[0].substitute_item_name,
        "Substitute Bottle SUB-A"
    );
    assert_eq!(all_relations[0].substitute_item_sku, "SUB-SUB-A");
    assert_close(all_relations[0].quantity, 6.0);

    let deleted = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/items/{main_id}/substitutes/{substitute_a}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let listed_after_delete = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{main_id}/substitutes"),
        &login.body.access_token,
    )
    .await;
    let listed_after_delete: Vec<SubstituteDetailResponse> = json_body(listed_after_delete).await;
    assert_eq!(listed_after_delete.len(), 1);
    assert_eq!(listed_after_delete[0].substitute_item_id, substitute_b);

    let delete_again = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/items/{main_id}/substitutes/{substitute_a}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete_again.status(), StatusCode::NOT_FOUND);
    assert_eq!(text_body(delete_again).await, "substitute_not_found");
}

#[tokio::test]
async fn substitutes_reject_invalid_targets_and_cycles() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let item_a = seed_item(&app, &login.body.access_token, "A").await;
    let item_b = seed_item(&app, &login.body.access_token, "B").await;

    let self_reference = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{item_a}/substitutes"),
        &login.body.access_token,
        &substitute_request(vec![(item_a, 1, None)]),
    )
    .await;
    assert_eq!(self_reference.status(), StatusCode::BAD_REQUEST);
    assert_eq!(text_body(self_reference).await, "invalid_request");

    let missing_target = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{item_a}/substitutes"),
        &login.body.access_token,
        &substitute_request(vec![(99_999, 1, None)]),
    )
    .await;
    assert_eq!(missing_target.status(), StatusCode::NOT_FOUND);
    assert_eq!(text_body(missing_target).await, "item_not_found");

    let bind_b_to_a = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{item_b}/substitutes"),
        &login.body.access_token,
        &substitute_request(vec![(item_a, 1, None)]),
    )
    .await;
    assert_eq!(bind_b_to_a.status(), StatusCode::OK);

    let cyclic = authorized_json_request(
        &app,
        "POST",
        &format!("/api/items/{item_a}/substitutes"),
        &login.body.access_token,
        &substitute_request(vec![(item_b, 1, None)]),
    )
    .await;
    assert_eq!(cyclic.status(), StatusCode::BAD_REQUEST);
    assert_eq!(text_body(cyclic).await, "invalid_request");

    let listed = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_a}/substitutes"),
        &login.body.access_token,
    )
    .await;
    let listed: Vec<SubstituteDetailResponse> = json_body(listed).await;
    assert!(listed.is_empty());
}

async fn seed_item(app: &crate::test_support::TestApp, access_token: &str, suffix: &str) -> i64 {
    let item = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: format!("Substitute Bottle {suffix}"),
            sku: format!("SUB-{suffix}"),
            category_id: None,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
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
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            source: "Substitute Supplier".to_owned(),
            notes: None,
            items: vec![InboundItemRequest {
                item_id,
                quantity,
                unit_price: 2.5,
                location: Some("S-01".to_owned()),
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some("2027-01-01".to_owned()),
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

fn substitute_request(items: Vec<(i64, i32, Option<&str>)>) -> SubstituteBindRequest {
    SubstituteBindRequest {
        substitutes: items
            .into_iter()
            .map(|(substitute_item_id, priority, notes)| SubstituteItem {
                substitute_item_id,
                priority,
                notes: notes.map(str::to_owned),
            })
            .collect(),
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
