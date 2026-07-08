//! stock 模块物品接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    stock::controller::{ItemCreateRequest, ItemResponse, ItemUpdateRequest},
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

    let forbidden_token = seed_viewer_and_login(&app).await;
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

async fn seed_viewer_and_login(app: &crate::test_support::TestApp) -> String {
    crate::test_support::seed_plain_user(app.state.database(), "viewer", "password").await;
    let rbac = crate::persistence::repository::RbacRepository::new(app.state.database());
    let viewer_role = rbac
        .ensure_role("viewer", "Viewer", None)
        .await
        .expect("viewer role should exist");
    let users = crate::persistence::repository::UserRepository::new(app.state.database());
    let viewer = users
        .find_by_username("viewer")
        .await
        .expect("viewer lookup should succeed")
        .expect("viewer should exist");
    rbac.assign_role_to_user(viewer.id, viewer_role)
        .await
        .expect("viewer role should assign");

    login_request(app, "viewer", "password")
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
