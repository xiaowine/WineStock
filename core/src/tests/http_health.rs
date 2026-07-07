//! 全局 HTTP 健康检查测试。

use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::{test_support::json_body, HealthResponse};

#[tokio::test]
async fn health_endpoint_returns_core_status_without_local_service() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body: HealthResponse = json_body(response).await;
    assert_eq!(body.status, "ok");
    assert_eq!(body.service, "winestock-core");
}
