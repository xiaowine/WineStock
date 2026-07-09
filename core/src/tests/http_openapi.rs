//! 全局 HTTP OpenAPI/Swagger 装配测试。

use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::{
    test_support::{error_code, json_body},
    OPENAPI_JSON_PATH,
};

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
    assert!(value["components"]["schemas"]["ApiErrorResponse"].is_object());
    assert!(value["components"]["securitySchemes"]["bearerAuth"].is_object());
    assert!(value["paths"]["/api/health"].is_object());
    assert!(value["paths"]["/api/health"]["get"]["security"].is_null());
    assert!(value["paths"]["/api/auth/register"].is_object());
    assert!(value["paths"]["/api/auth/login"].is_object());
    assert!(value["paths"]["/api/auth/refresh"].is_object());
    assert!(value["paths"]["/api/auth/logout"].is_object());
    assert!(value["paths"]["/api/auth/me"].is_object());
    assert!(value["paths"]["/api/auth/me/password"].is_object());
    assert!(value["paths"]["/api/users"].is_object());
    assert!(value["paths"]["/api/users/{id}"].is_object());
    assert!(value["paths"]["/api/users/{id}/status"].is_object());
    assert!(value["paths"]["/api/users/{id}/permissions"].is_object());
    assert!(value["paths"]["/api/users/{id}/password"].is_object());
    assert!(value["paths"]["/api/permissions"].is_object());
    assert!(value["paths"]["/api/users/{id}/roles"].is_null());
    assert!(value["paths"]["/api/roles"].is_null());
    assert!(value["paths"]["/api/templates"].is_object());
    assert!(value["paths"]["/api/templates/{id}"].is_object());
    assert!(value["paths"]["/api/templates/{id}/copy"].is_object());
    assert!(value["paths"]["/api/items"].is_object());
    assert!(value["paths"]["/api/items/filter-values"].is_object());
    assert!(value["paths"]["/api/items/{id}"].is_object());
    assert!(value["paths"]["/api/location-groups/tree"].is_object());
    assert!(value["paths"]["/api/location-groups"].is_object());
    assert!(value["paths"]["/api/location-groups/{id}"].is_object());
    assert!(value["paths"]["/api/locations"].is_object());
    assert!(value["paths"]["/api/locations/{id}"].is_object());
    assert!(value["paths"]["/api/location-transfers"].is_object());
    assert!(value["paths"]["/api/substitutes"].is_object());
    assert!(value["paths"]["/api/substitutes/{item_id}"].is_object());
    assert!(value["paths"]["/api/substitutes/{item_id}/{substitute_item_id}"].is_object());
    let legacy_child_segment = concat!("sub", "stitutes");
    let legacy_item_child_path = format!("/api/items/{{id}}/{legacy_child_segment}");
    let legacy_relation_param = ["substitute", "id"].join("_");
    let legacy_item_substitute_relation_path =
        format!("{legacy_item_child_path}/{{{legacy_relation_param}}}");
    assert!(value["paths"][&legacy_item_child_path].is_null());
    assert!(value["paths"][&legacy_item_substitute_relation_path].is_null());
    assert!(value["paths"]["/api/inbound"].is_object());
    assert!(value["paths"]["/api/inbound/filter-values"].is_object());
    assert!(value["paths"]["/api/inbound/{id}"].is_object());
    assert!(value["paths"]["/api/inbound/{id}/approve"].is_null());
    assert!(value["paths"]["/api/inbound/{id}/reject"].is_null());
    assert!(value["paths"]["/api/outbound"].is_object());
    assert!(value["paths"]["/api/outbound/filter-values"].is_object());
    assert!(value["paths"]["/api/outbound/{id}"].is_object());
    assert!(value["paths"]["/api/outbound/{id}/approve"].is_null());
    assert!(value["paths"]["/api/outbound/{id}/reject"].is_null());
    assert!(value["paths"]["/api/stock-approvals/inbound/{id}/approve"].is_object());
    assert!(value["paths"]["/api/stock-approvals/inbound/{id}/reject"].is_object());
    assert!(value["paths"]["/api/stock-approvals/outbound/{id}/approve"].is_object());
    assert!(value["paths"]["/api/stock-approvals/outbound/{id}/reject"].is_object());
    assert!(value["paths"]["/api/dashboard/overview"].is_object());
    assert!(value["paths"]["/api/dashboard/trends"].is_object());
    assert!(value["paths"]["/api/events"].is_object());

    let tags = value["tags"]
        .as_array()
        .expect("openapi tags should be an array")
        .iter()
        .map(|tag| {
            tag["name"]
                .as_str()
                .expect("tag name should be a string")
                .to_owned()
        })
        .collect::<Vec<_>>();
    for expected in [
        "health",
        "auth",
        "users",
        "templates",
        "items",
        "locations",
        "inbound",
        "outbound",
        "stock-approvals",
        "substitutes",
        "dashboard",
        "events",
    ] {
        assert!(
            tags.iter().any(|tag| tag == expected),
            "{expected} tag should be declared"
        );
    }
    assert!(
        tags.iter().all(|tag| tag != "stock"),
        "stock tag should be split into business tags"
    );
    assert_operation_tag(&value, "/api/health", "get", "health");
    assert_operation_tag(&value, "/api/templates", "post", "templates");
    assert_operation_tag(&value, "/api/items", "post", "items");
    assert_operation_tag(&value, "/api/location-groups/tree", "get", "locations");
    assert_operation_tag(&value, "/api/inbound", "post", "inbound");
    assert_operation_tag(
        &value,
        "/api/stock-approvals/inbound/{id}/approve",
        "post",
        "stock-approvals",
    );
    assert_operation_tag(&value, "/api/outbound", "post", "outbound");
    assert_operation_tag(
        &value,
        "/api/stock-approvals/outbound/{id}/approve",
        "post",
        "stock-approvals",
    );
    assert_operation_tag(&value, "/api/substitutes", "get", "substitutes");
    assert_operation_tag(&value, "/api/dashboard/overview", "get", "dashboard");
    assert_operation_tag(&value, "/api/events", "get", "events");
    assert_no_operation_tag(&value, "stock");
}

#[tokio::test]
async fn health_endpoint_is_available_without_local_service_state() {
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
    let value: serde_json::Value = json_body(response).await;
    assert_eq!(value["status"], "OK");
}

#[tokio::test]
async fn unknown_route_returns_json_error() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/missing")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(error_code(response).await, "not_found");
}

#[tokio::test]
async fn unsupported_method_returns_json_error() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(
        response.status(),
        axum::http::StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(error_code(response).await, "method_not_allowed");
}

fn assert_operation_tag(value: &serde_json::Value, path: &str, method: &str, expected: &str) {
    let operation = &value["paths"][path][method];
    let tags = operation["tags"]
        .as_array()
        .expect("operation tags should be an array");
    assert!(
        tags.iter()
            .any(|tag| tag.as_str().is_some_and(|tag| tag == expected)),
        "{method} {path} should use {expected} tag"
    );
}

fn assert_no_operation_tag(value: &serde_json::Value, unexpected: &str) {
    let paths = value["paths"]
        .as_object()
        .expect("openapi paths should be an object");
    for (path, path_item) in paths {
        let Some(methods) = path_item.as_object() else {
            continue;
        };
        for (method, operation) in methods {
            let Some(tags) = operation["tags"].as_array() else {
                continue;
            };
            assert!(
                tags.iter()
                    .all(|tag| tag.as_str().is_none_or(|tag| tag != unexpected)),
                "{method} {path} should not use {unexpected} tag"
            );
        }
    }
}
