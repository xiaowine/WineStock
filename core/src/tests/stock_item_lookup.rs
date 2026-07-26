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
    price_request: Arc<Mutex<Option<Value>>>,
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
    assert_eq!(candidate.parameters.len(), 1);
    assert_eq!(table_count(&app, "stock_items").await, item_count);
    assert_eq!(table_count(&app, "audit_events").await, event_count);

    let (headers, body) = mock
        .request
        .lock()
        .expect("request lock should work")
        .clone()
        .expect("upstream request should exist");
    assert_eq!(body["attributes"], json!({}));
    assert_eq!(body["path"], "0819f05c4eef4c71ace90d822a990e87");
    assert_eq!(body["uid"], "0819f05c4eef4c71ace90d822a990e87");
    assert_eq!(body["page"], 1);
    assert_eq!(body["pageSize"], 50);
    assert_eq!(body["tag"], json!([]));
    assert_eq!(body["wd"], "C2983288");
    assert!(body.get("codes").is_none());
    assert!(headers.get("cookie").is_none());
    assert!(headers.get("origin").is_none());
    assert!(headers.get("referer").is_none());
    let price_body = mock
        .price_request
        .lock()
        .expect("price request lock should work")
        .clone()
        .expect("price request should exist");
    assert_eq!(price_body["numbers"], json!(["C2983288"]));
    assert_eq!(price_body["path"], "0819f05c4eef4c71ace90d822a990e87");
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
        json!({ "success": true, "code": 0, "result": { "lists": {} } }),
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

async fn spawn_mock_server(state: MockState, status: StatusCode, body: Value) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener should bind");
    let address = listener.local_addr().expect("mock address should exist");
    let search_body = body.clone();
    let router = Router::new()
        .route(
            "/api/devices/search",
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
        .route(
            "/api/components/getSmtPartInfo",
            post(
                |State(state): State<MockState>, Json(request): Json<Value>| async move {
                    *state
                        .price_request
                        .lock()
                        .expect("price request lock should work") = Some(request);
                    (
                        StatusCode::OK,
                        Json(json!({
                            "success": true,
                            "code": 0,
                            "result": [{
                                "component_code": "C2983288",
                                "onSale": 1,
                                "stock_num": 10,
                                "priceList": [
                                    { "startNumber": 10, "productPrice": 8.2 },
                                    { "startNumber": 1, "productPrice": 9.91 }
                                ]
                            }]
                        })),
                    )
                },
            ),
        )
        .with_state(state);
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock server should run");
    });
    format!("http://{address}/api/devices/search")
}

fn success_body() -> Value {
    json!({
        "success": true,
        "code": 0,
        "result": { "lists": { "lcsc": [{
            "product_code": "C2983288",
            "description": "旋转编码开关",
            "attributes": {
                "LCSC Part Name": "旋转编码开关",
                "Supplier Part": "C2983288",
                "Manufacturer": "SM Switch",
                "Manufacturer Part": "BER-04",
                "Supplier Footprint": "插件",
                "Datasheet": "https://example.com/BER-04.pdf",
                "Symbol": "private-symbol-id",
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

async fn table_count(app: &crate::test_support::TestApp, table: &str) -> i64 {
    let row = app
        .state
        .database()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) AS count FROM {table}"),
        ))
        .await
        .expect("table count should query")
        .expect("table count should exist");
    row.try_get("", "count").expect("count should decode")
}
