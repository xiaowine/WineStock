//! stock 模块模板接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        ItemCreateRequest, TemplateCopyRequest, TemplateCreateRequest, TemplateFieldDef,
        TemplateFieldType, TemplateResponse, TemplateUpdateRequest,
    },
    test_support::{json_body, login_request, seeded_app, text_body},
};

#[tokio::test]
async fn template_crud_copy_and_delete_follow_business_rules() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    assert_eq!(login.status, StatusCode::OK);

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/templates",
        &login.body.access_token,
        &template_request("Raw Material"),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let template: TemplateResponse = json_body(created).await;
    assert_eq!(template.name, "Raw Material");
    assert_eq!(template.fields.len(), 3);
    assert_eq!(template.fields[0].field_name, "brand");
    assert_eq!(template.fields[1].field_type, TemplateFieldType::Number);
    assert_eq!(
        template.fields[2].options.as_deref(),
        Some(&["red".to_owned(), "white".to_owned()][..])
    );

    let duplicate = authorized_json_request(
        &app,
        "POST",
        "/api/templates",
        &login.body.access_token,
        &template_request("Raw Material"),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(duplicate).await, "template_name_taken");

    let listed =
        authorized_empty_request(&app, "GET", "/api/templates", &login.body.access_token).await;
    assert_eq!(listed.status(), StatusCode::OK);
    let templates: Vec<TemplateResponse> = json_body(listed).await;
    assert_eq!(templates.len(), 1);

    let updated = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/templates/{}", template.id),
        &login.body.access_token,
        &TemplateUpdateRequest {
            name: Some("Finished Wine".to_owned()),
            description: Some("Updated template".to_owned()),
            fields: Some(vec![TemplateFieldDef {
                field_name: "vintage".to_owned(),
                field_type: TemplateFieldType::Number,
                required: Some(true),
                searchable: Some(true),
                options: None,
                default_value: Some("2026".to_owned()),
            }]),
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: TemplateResponse = json_body(updated).await;
    assert_eq!(updated.name, "Finished Wine");
    assert_eq!(updated.fields.len(), 1);
    assert_eq!(updated.fields[0].field_name, "vintage");

    let copied = authorized_json_request(
        &app,
        "POST",
        &format!("/api/templates/{}/copy", template.id),
        &login.body.access_token,
        &TemplateCopyRequest {
            name: "Finished Wine Copy".to_owned(),
        },
    )
    .await;
    assert_eq!(copied.status(), StatusCode::CREATED);
    let copied: TemplateResponse = json_body(copied).await;
    assert_eq!(copied.name, "Finished Wine Copy");
    assert_eq!(copied.fields.len(), 1);

    let item = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "Bottle".to_owned(),
            sku: "BOT-001".to_owned(),
            category_id: Some(template.id),
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
        },
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);

    let delete_in_use = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/templates/{}", template.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(delete_in_use.status(), StatusCode::CONFLICT);
    assert_eq!(text_body(delete_in_use).await, "template_in_use");

    let deleted = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/templates/{}", copied.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let missing = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/templates/{}", copied.id),
        &login.body.access_token,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn template_validation_and_authorization_fail_before_write() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let select_without_options = authorized_json_request(
        &app,
        "POST",
        "/api/templates",
        &login.body.access_token,
        &TemplateCreateRequest {
            name: "Invalid Select".to_owned(),
            description: None,
            fields: vec![TemplateFieldDef {
                field_name: "kind".to_owned(),
                field_type: TemplateFieldType::Select,
                required: None,
                searchable: None,
                options: None,
                default_value: None,
            }],
        },
    )
    .await;
    assert_eq!(select_without_options.status(), StatusCode::BAD_REQUEST);

    let duplicate_fields = authorized_json_request(
        &app,
        "POST",
        "/api/templates",
        &login.body.access_token,
        &TemplateCreateRequest {
            name: "Duplicate Fields".to_owned(),
            description: None,
            fields: vec![
                TemplateFieldDef {
                    field_name: "brand".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: None,
                    searchable: None,
                    options: None,
                    default_value: None,
                },
                TemplateFieldDef {
                    field_name: " Brand ".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: None,
                    searchable: None,
                    options: None,
                    default_value: None,
                },
            ],
        },
    )
    .await;
    assert_eq!(duplicate_fields.status(), StatusCode::BAD_REQUEST);

    let forbidden_token =
        seed_user_with_permissions_and_login(&app, "template-viewer", &["stock.read"]).await;
    let forbidden = authorized_json_request(
        &app,
        "POST",
        "/api/templates",
        &forbidden_token,
        &template_request("Viewer Template"),
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

fn template_request(name: &str) -> TemplateCreateRequest {
    TemplateCreateRequest {
        name: name.to_owned(),
        description: Some("Template for tests".to_owned()),
        fields: vec![
            TemplateFieldDef {
                field_name: "brand".to_owned(),
                field_type: TemplateFieldType::Text,
                required: Some(true),
                searchable: Some(true),
                options: None,
                default_value: None,
            },
            TemplateFieldDef {
                field_name: "abv".to_owned(),
                field_type: TemplateFieldType::Number,
                required: Some(false),
                searchable: Some(false),
                options: None,
                default_value: Some("13.5".to_owned()),
            },
            TemplateFieldDef {
                field_name: "style".to_owned(),
                field_type: TemplateFieldType::Select,
                required: Some(false),
                searchable: Some(true),
                options: Some(vec!["red".to_owned(), "white".to_owned()]),
                default_value: Some("red".to_owned()),
            },
        ],
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
