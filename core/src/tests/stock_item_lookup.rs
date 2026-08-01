//! 立创单商品资料查询 HTTP 契约测试。

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    routing::post,
    Json, Router,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::{
    persistence::repository::{RbacRepository, UserRepository},
    stock::{controller::LcscItemLookupResponse, permissions::STOCK_ITEM_READ_PERMISSION},
    test_support::{
        empty_app_with_mock_lcsc, error_code, json_body, login_request, seed_plain_user,
    },
};

#[derive(Clone, Default)]
struct MockState {
    request: Arc<Mutex<Option<(HeaderMap, Value)>>>,
}

#[tokio::test]
async fn lookup_uses_fixed_search_request_and_returns_normalized_candidate() {
    let mock = MockState::default();
    let endpoint = spawn_mock_server(mock.clone(), StatusCode::OK, success_body()).await;
    let app = empty_app_with_mock_lcsc(endpoint).await;
    seed_admin(&app).await;
    let login = login_request(&app, "admin", "password").await;
    let item_count = table_count(&app, "stock_items").await;
    let event_count = table_count(&app, "audit_events").await;

    let response = authorized_get(
        &app,
        "/api/items/lookups/lcsc/C2983288",
        &login.body.access_token,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let candidate: LcscItemLookupResponse = json_body(response).await;
    assert_eq!(candidate.product_code, "C2983288");
    assert_eq!(candidate.name, "BER-04");
    assert_eq!(candidate.manufacturer.as_deref(), Some("SM Switch"));
    assert_eq!(candidate.default_price, Some(9.91));
    assert_eq!(
        candidate.image_url.as_deref(),
        Some("https://alimg.szlcsc.com/upload/public/product/source/20241118/example.jpg")
    );
    assert_eq!(candidate.parameters.len(), 1);
    assert_eq!(table_count(&app, "stock_items").await, item_count);
    assert_eq!(table_count(&app, "audit_events").await, event_count);

    let (headers, body) = mock
        .request
        .lock()
        .expect("request lock should work")
        .clone()
        .expect("upstream request should exist");
    assert_eq!(body["keyword"], "C2983288");
    assert_eq!(body["pageSize"], 10);
    assert_eq!(body["currentPage"], 1);
    assert_eq!(body["searchSource"], "main_so");
    assert_eq!(body["asyncRequest"], false);
    assert_eq!(body.as_object().map(|object| object.len()), Some(5));
    assert!(headers.get("cookie").is_none());
    assert!(headers.get("origin").is_none());
    assert!(headers.get("referer").is_none());
}

#[tokio::test]
async fn lookup_validates_permissions_input_and_upstream_errors() {
    let endpoint = spawn_mock_server(MockState::default(), StatusCode::OK, success_body()).await;
    let app = empty_app_with_mock_lcsc(endpoint).await;
    seed_admin(&app).await;
    seed_plain_user(app.state.database(), "reader", "password").await;
    assign_permission(&app, "reader", STOCK_ITEM_READ_PERMISSION).await;
    let reader = login_request(&app, "reader", "password").await;
    let admin = login_request(&app, "admin", "password").await;

    let unauthorized = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/items/lookups/lcsc/C2983288")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let forbidden = authorized_get(
        &app,
        "/api/items/lookups/lcsc/C2983288",
        &reader.body.access_token,
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let invalid = authorized_get(
        &app,
        "/api/items/lookups/lcsc/not-a-code",
        &admin.body.access_token,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid).await, "invalid_lcsc_product_code");

    let missing_endpoint = spawn_mock_server(
        MockState::default(),
        StatusCode::OK,
        json!({
            "code": 200,
            "ok": true,
            "result": { "searchResult": { "productRecordList": [] } }
        }),
    )
    .await;
    let missing_app = empty_app_with_mock_lcsc(missing_endpoint).await;
    seed_admin(&missing_app).await;
    let missing_login = login_request(&missing_app, "admin", "password").await;
    let missing = authorized_get(
        &missing_app,
        "/api/items/lookups/lcsc/C2983288",
        &missing_login.body.access_token,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(missing).await, "lcsc_product_not_found");

    let failed_endpoint =
        spawn_mock_server(MockState::default(), StatusCode::BAD_GATEWAY, json!({})).await;
    let failed_app = empty_app_with_mock_lcsc(failed_endpoint).await;
    seed_admin(&failed_app).await;
    let failed_login = login_request(&failed_app, "admin", "password").await;
    let failed = authorized_get(
        &failed_app,
        "/api/items/lookups/lcsc/C2983288",
        &failed_login.body.access_token,
    )
    .await;
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(error_code(failed).await, "lcsc_lookup_failed");
}

#[tokio::test]
async fn batch_lookup_returns_per_code_results_and_falls_back_for_missing_codes() {
    let endpoint = spawn_mock_server(MockState::default(), StatusCode::OK, success_body()).await;
    let app = empty_app_with_mock_lcsc(endpoint).await;
    seed_admin(&app).await;
    let login = login_request(&app, "admin", "password").await;

    let response = authorized_post(
        &app,
        "/api/items/lookups/lcsc",
        &login.body.access_token,
        json!({ "product_codes": [" c2983288 ", "not-a-code", "C000"] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = json_body(response).await;
    let results = body["results"]
        .as_array()
        .expect("results should be an array");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["product_code"], "C2983288");
    assert_eq!(results[0]["candidate"]["product_code"], "C2983288");
    assert!(results[0]["error"].is_null());
    assert_eq!(results[1]["error"], "invalid_product_code");
    assert!(results[1]["candidate"].is_null());
    assert_eq!(results[2]["error"], "product_not_found");
    assert!(results[2]["candidate"].is_null());
}

#[tokio::test]
async fn batch_lookup_rejects_more_than_ten_codes() {
    let endpoint = spawn_mock_server(MockState::default(), StatusCode::OK, success_body()).await;
    let app = empty_app_with_mock_lcsc(endpoint).await;
    seed_admin(&app).await;
    let login = login_request(&app, "admin", "password").await;

    let response = authorized_post(
        &app,
        "/api/items/lookups/lcsc",
        &login.body.access_token,
        json!({ "product_codes": [
            "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "C11"
        ] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "invalid_request");
}

async fn spawn_mock_server(state: MockState, status: StatusCode, body: Value) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener should bind");
    let address = listener.local_addr().expect("mock address should exist");
    let search_body = body.clone();
    let router = Router::new()
        .route(
            "/phone/global/query",
            post(
                move |State(state): State<MockState>,
                      headers: HeaderMap,
                      Json(request): Json<Value>| {
                    let body = search_body.clone();
                    async move {
                        *state.request.lock().expect("request lock should work") =
                            Some((headers, request));
                        (status, Json(body))
                    }
                },
            ),
        )
        .with_state(state);
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock server should run");
    });
    format!("http://{address}/phone/global/query")
}

fn success_body() -> Value {
    json!({
        "code": 200,
        "result": { "searchResult": { "productRecordList": [{
            "productVO": {
                "productCode": "C2983288",
                "productName": "旋转编码开关",
                "productGradePlateName": "SM Switch",
                "encapsulationModel": "插件",
                "productModel": "BER-04",
                "breviaryImageUrl": "https://alimg.szlcsc.com/upload/public/product/breviary/20241118/fallback.jpg",
                "bigImageUrl": "https://alimg.szlcsc.com/upload/public/product/middle/20241118/example.jpg",
                "luceneBreviaryImageUrls": "https://alimg.szlcsc.com/upload/public/product/breviary/20241118/example.jpg<$>https://alimg.szlcsc.com/upload/public/product/breviary/20241118/second.jpg",
                "stockNumber": 10,
                "productPriceList": [
                    { "startPurchasedNumber": 10, "productPrice": 8.2 },
                    { "startPurchasedNumber": 1, "productPrice": 9.91 }
                ],
                "fileTypeVOList": [{
                    "fileType": "pdf_property",
                    "detailVOList": [{
                        "fileUrl": "/upload/public/pdf/source/20241012/BER-04.pdf"
                    }]
                }]
            },
            "lightProductIntro": "旋转编码开关",
            "lightBrandName": "SM Switch",
            "lightProductModel": "BER-04",
            "lightStandard": "插件",
            "paramLinkedMap": {
                "Operating Temperature": "-40℃~+85℃"
            }
        }] } }
    })
}

async fn seed_admin(app: &crate::test_support::TestApp) {
    seed_plain_user(app.state.database(), "admin", "password").await;
    let users = UserRepository::new(app.state.database());
    let rbac = RbacRepository::new(app.state.database());
    let admin = users
        .find_by_username("admin")
        .await
        .expect("admin query should work")
        .expect("admin should exist");
    for permission in crate::rbac::builtin_permission_codes() {
        let permission_id = rbac
            .ensure_permission(&permission, None)
            .await
            .expect("permission should exist");
        rbac.assign_permission_to_user(admin.id, permission_id)
            .await
            .expect("permission should assign");
    }
}

async fn assign_permission(app: &crate::test_support::TestApp, username: &str, permission: &str) {
    let users = UserRepository::new(app.state.database());
    let rbac = RbacRepository::new(app.state.database());
    let user = users
        .find_by_username(username)
        .await
        .expect("user query should work")
        .expect("user should exist");
    let permission_id = rbac
        .ensure_permission(permission, None)
        .await
        .expect("permission should exist");
    rbac.assign_permission_to_user(user.id, permission_id)
        .await
        .expect("permission should assign");
}

async fn authorized_get(
    app: &crate::test_support::TestApp,
    path: &str,
    token: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

async fn authorized_post(
    app: &crate::test_support::TestApp,
    path: &str,
    token: &str,
    body: Value,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
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
        .expect("table count should query")
        .expect("table count should exist");
    row.try_get("", "count").expect("count should decode")
}
