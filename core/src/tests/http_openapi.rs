//! 全局 HTTP OpenAPI/Swagger 装配测试。

use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::{test_support::json_body, OPENAPI_JSON_PATH};

#[tokio::test]
async fn openapi_includes_bearer_auth_and_auth_paths() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(OPENAPI_JSON_PATH)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let value: serde_json::Value = json_body(response).await;
    assert!(value["components"]["securitySchemes"]["bearerAuth"].is_object());
    assert!(value["paths"]["/api/auth/register"].is_object());
    assert!(value["paths"]["/api/auth/login"].is_object());
    assert!(value["paths"]["/api/auth/refresh"].is_object());
    assert!(value["paths"]["/api/auth/logout"].is_object());
    assert!(value["paths"]["/api/auth/me"].is_object());
}
