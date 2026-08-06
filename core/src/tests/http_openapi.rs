//! 全局 HTTP OpenAPI/Swagger 装配测试。

use axum::{
    body::Body,
    http::{
        header::{
            ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
            ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE, CONTENT_TYPE, ORIGIN,
        },
        Request,
    },
};
use tower::ServiceExt;

use crate::test_support::{empty_app, error_code, json_body};
#[cfg(debug_assertions)]
use crate::OPENAPI_JSON_PATH;
#[cfg(debug_assertions)]
use crate::SWAGGER_UI_PATH;

#[cfg(debug_assertions)]
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
    assert_eq!(response.headers()[ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    let value: serde_json::Value = json_body(response).await;
    assert!(value["components"]["schemas"]["ApiErrorResponse"].is_object());
    assert!(value["components"]["schemas"]["LocationGroupTreeNode"].is_object());
    assert!(
        value["components"]["schemas"]["UserAdminResponse"]["properties"]["display_name"].is_null()
    );
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
    assert!(value["paths"]["/api/users/{id}"]["delete"].is_object());
    assert!(value["paths"]["/api/users/{id}/status"].is_object());
    assert!(value["paths"]["/api/users/{id}/permissions"].is_object());
    assert!(value["paths"]["/api/users/{id}/password"].is_object());
    assert!(value["paths"]["/api/permissions"].is_object());
    assert!(value["paths"]["/api/files/images"].is_object());
    assert!(value["paths"]["/api/files/{id}"]["get"].is_object());
    assert!(value["paths"]["/api/files/{id}"]["delete"].is_object());
    assert!(value["paths"]["/api/users/{id}/roles"].is_null());
    assert!(value["paths"]["/api/roles"].is_null());
    assert!(value["paths"]["/api/item-categories"].is_object());
    assert!(value["paths"]["/api/item-attribute-templates"].is_object());
    assert!(
        value["components"]["schemas"]["ItemCategoryResponse"]["properties"]["item_usage_count"]
            .is_object()
    );
    assert!(
        value["components"]["schemas"]["ItemAttributeTemplateResponse"]["properties"]
            ["item_usage_count"]
            .is_object()
    );
    assert!(value["paths"]["/api/item-categories/{id}"]["delete"]["responses"]["200"].is_object());
    assert!(
        value["paths"]["/api/item-attribute-templates/{id}"]["delete"]["responses"]["200"]
            .is_object()
    );
    assert!(value["paths"]["/api/item-attribute-templates/{id}/copy"].is_object());
    assert!(value["paths"]["/api/inbound-templates"].is_null());
    assert!(value["paths"]["/api/items"].is_object());
    assert!(value["paths"]["/api/items/options"].is_object());
    assert!(value["paths"]["/api/items/filter-values"].is_object());
    assert!(value["paths"]["/api/items/lookups/lcsc/{product_code}"].is_object());
    assert!(value["paths"]["/api/items/lookups/lcsc/{product_code}/image"].is_null());
    assert!(value["components"]["schemas"]["LcscItemLookupResponse"].is_object());
    assert!(value["paths"]["/api/items"]["get"]["parameters"]
        .as_array()
        .is_some_and(|parameters| parameters
            .iter()
            .any(|parameter| parameter["name"] == "filters")));
    assert!(
        value["paths"]["/api/items/filter-values"]["get"]["parameters"]
            .as_array()
            .is_some_and(|parameters| parameters
                .iter()
                .any(|parameter| parameter["name"] == "filters"))
    );
    assert!(value["paths"]["/api/items/{id}"].is_object());
    assert!(value["paths"]["/api/items/{id}/inventory"].is_object());
    assert!(value["paths"]["/api/items/{id}/batches"].is_object());
    assert!(value["components"]["schemas"]["ItemResponse"].is_null());
    assert!(value["components"]["schemas"]["ItemCatalogPageResponse"].is_object());
    assert!(value["components"]["schemas"]["ItemMutationResponse"].is_object());
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
    assert!(value["paths"]["/api/inbound"]["get"]["parameters"]
        .as_array()
        .is_some_and(|parameters| parameters
            .iter()
            .any(|parameter| parameter["name"] == "status")));
    assert!(value["paths"]["/api/outbound"].is_object());
    assert!(value["paths"]["/api/outbound"]["get"]["parameters"]
        .as_array()
        .is_some_and(|parameters| parameters
            .iter()
            .any(|parameter| parameter["name"] == "status")));
    assert_eq!(
        value["components"]["schemas"]["InboundSubmissionMode"]["enum"],
        serde_json::json!(["pending_approval", "direct"])
    );
    assert_eq!(
        value["components"]["schemas"]["ItemAttributeUnitMode"]["enum"],
        serde_json::json!(["none", "fixed", "select"])
    );
    assert!(
        value["components"]["schemas"]["ItemAttributeTemplateFieldDef"]["properties"]
            ["catalog_visible"]
            .is_object()
    );
    for schema in ["ItemAttributeRequest", "ItemAttributeResponse"] {
        let properties = &value["components"]["schemas"][schema]["properties"];
        assert!(properties["definition_id"].is_object());
        assert!(properties["template_field_id"].is_null());
    }
    assert!(
        value["components"]["schemas"]["ItemAttributeTemplateFieldDef"]["properties"]["unit"]
            .is_object()
    );
    assert!(
        value["components"]["schemas"]["ItemAttributeTemplateFieldResponse"]["allOf"][1]
            ["properties"]["unit"]
            .is_object()
    );
    assert!(value["components"]["schemas"]["TemplateFieldDef"]["properties"]["unit"].is_null());
    assert!(
        value["components"]["schemas"]["InboundCreateRequest"]["properties"]["submission_mode"]
            .is_object()
    );
    assert!(
        value["components"]["schemas"]["InboundResponse"]["properties"]["submission_mode"]
            .is_object()
    );
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
        "files",
        "item-categories",
        "item-attribute-templates",
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
    assert_operation_tag(&value, "/api/item-categories", "post", "item-categories");
    assert_operation_tag(
        &value,
        "/api/item-attribute-templates",
        "post",
        "item-attribute-templates",
    );
    assert_operation_tag(&value, "/api/files/images", "post", "files");
    assert_operation_tag(&value, "/api/items", "post", "items");
    assert_operation_tag(&value, "/api/items/lookups/lcsc", "post", "items");
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
    assert_operation_response(&value, "/api/auth/login", "post", "400");
    assert_operation_response(&value, "/api/auth/refresh", "post", "400");
    assert_operation_response(&value, "/api/auth/logout", "post", "400");
    assert_operation_response(&value, "/api/items/{id}", "get", "400");
    assert_operation_response(&value, "/api/events", "get", "400");
    assert_no_operation_response(&value, "/api/health", "get", "400");
    assert_location_tree_schema(&value);
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn swagger_ui_is_available_in_debug_builds() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{SWAGGER_UI_PATH}/"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(response.headers()[CONTENT_TYPE]
        .to_str()
        .expect("content type should be valid")
        .starts_with("text/html"));
}

#[cfg(debug_assertions)]
#[cfg(not(debug_assertions))]
#[tokio::test]
async fn swagger_ui_is_not_available_without_swagger_feature() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/swagger-ui/")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    assert_eq!(error_code(response).await, "not_found");
}

#[cfg(not(debug_assertions))]
#[tokio::test]
async fn openapi_json_is_not_available_in_release_builds() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    assert_eq!(error_code(response).await, "not_found");
}

#[cfg(not(debug_assertions))]
#[tokio::test]
async fn swagger_ui_is_not_available_in_release_builds() {
    let response = crate::build_router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/swagger-ui/")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    assert_eq!(error_code(response).await, "not_found");
}

#[tokio::test]
async fn cors_preflight_covers_merged_business_routes() {
    let app = empty_app().await;
    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/auth/login")
                .header(ORIGIN, "http://localhost:5173")
                .header("access-control-request-method", "POST")
                .header(
                    "access-control-request-headers",
                    "authorization,content-type",
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
    assert_eq!(response.headers()[ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert_eq!(
        response.headers()[ACCESS_CONTROL_ALLOW_METHODS],
        "GET,POST,PUT,PATCH,DELETE,OPTIONS"
    );
    assert_eq!(
        response.headers()[ACCESS_CONTROL_ALLOW_HEADERS],
        "authorization,content-type,accept"
    );
    assert_eq!(response.headers()[ACCESS_CONTROL_MAX_AGE], "86400");
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

#[cfg(debug_assertions)]
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

#[cfg(debug_assertions)]
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

#[cfg(debug_assertions)]
fn assert_operation_response(value: &serde_json::Value, path: &str, method: &str, status: &str) {
    assert!(
        value["paths"][path][method]["responses"][status].is_object(),
        "{method} {path} should document {status}"
    );
}

#[cfg(debug_assertions)]
fn assert_no_operation_response(value: &serde_json::Value, path: &str, method: &str, status: &str) {
    assert!(
        value["paths"][path][method]["responses"][status].is_null(),
        "{method} {path} should not document {status}"
    );
}

#[cfg(debug_assertions)]
fn assert_location_tree_schema(value: &serde_json::Value) {
    let schema = &value["paths"]["/api/location-groups/tree"]["get"]["responses"]["200"]["content"]
        ["application/json"]["schema"];

    assert_eq!(schema["type"], "array");
    assert_eq!(
        schema["items"]["$ref"],
        "#/components/schemas/LocationGroupTreeNode"
    );
}
