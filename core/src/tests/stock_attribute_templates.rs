//! 分类、物品属性模板和入库模板独立接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundTemplateCreateRequest, InboundTemplateResponse, ItemAttributeTemplateCreateRequest,
        ItemAttributeTemplateFieldDef, ItemAttributeTemplateResponse,
        ItemAttributeTemplateUpdateRequest, ItemAttributeUnitMode, ItemAttributeUnitRule,
        ItemCategoryCreateRequest, ItemCategoryResponse, TemplateCopyRequest, TemplateFieldDef,
        TemplateFieldType,
    },
    test_support::{error_code, json_body, login_request, seeded_app},
};

#[tokio::test]
async fn category_and_two_template_kinds_are_independent() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let token = &login.body.access_token;

    let category = authorized_json(
        &app,
        "POST",
        "/api/item-categories",
        token,
        &ItemCategoryCreateRequest {
            name: "测试分类".to_owned(),
            description: Some("只负责归类".to_owned()),
            sort_order: Some(20),
        },
    )
    .await;
    assert_eq!(category.status(), StatusCode::CREATED);
    let category: ItemCategoryResponse = json_body(category).await;

    let inbound = authorized_json(
        &app,
        "POST",
        "/api/inbound-templates",
        token,
        &InboundTemplateCreateRequest {
            name: "测试收货".to_owned(),
            description: None,
            fields: vec![field("收货照片", TemplateFieldType::File, false)],
        },
    )
    .await;
    assert_eq!(inbound.status(), StatusCode::CREATED);
    let inbound: InboundTemplateResponse = json_body(inbound).await;

    let item = authorized_json(
        &app,
        "POST",
        "/api/item-attribute-templates",
        token,
        &ItemAttributeTemplateCreateRequest {
            name: "测试物品属性".to_owned(),
            description: None,
            default_inbound_template_id: Some(inbound.id),
            fields: vec![item_field("型号", TemplateFieldType::Text, true)],
        },
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);
    let item: ItemAttributeTemplateResponse = json_body(item).await;
    assert_eq!(item.default_inbound_template_id, Some(inbound.id));
    assert_eq!(item.fields[0].field.field_name, "型号");

    let categories = authorized_empty(&app, "GET", "/api/item-categories", token).await;
    let categories: Vec<ItemCategoryResponse> = json_body(categories).await;
    assert!(categories.iter().any(|entry| entry.id == category.id));
    let inbound_templates: Vec<InboundTemplateResponse> =
        json_body(authorized_empty(&app, "GET", "/api/inbound-templates", token).await).await;
    assert!(inbound_templates.iter().any(|entry| entry.id == inbound.id));
    let item_templates: Vec<ItemAttributeTemplateResponse> =
        json_body(authorized_empty(&app, "GET", "/api/item-attribute-templates", token).await)
            .await;
    assert!(item_templates.iter().any(|entry| entry.id == item.id));
}

#[tokio::test]
async fn template_validation_copy_and_permissions_are_enforced() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let token = &login.body.access_token;
    let invalid = authorized_json(
        &app,
        "POST",
        "/api/inbound-templates",
        token,
        &InboundTemplateCreateRequest {
            name: "坏模板".to_owned(),
            description: None,
            fields: vec![TemplateFieldDef {
                field_name: "状态".to_owned(),
                field_type: TemplateFieldType::Select,
                required: None,
                searchable: None,
                options: None,
                default_value: None,
            }],
        },
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    for (name, field_type, default_value) in [
        ("坏日期默认值", TemplateFieldType::Date, "2026-02-31"),
        ("坏图片默认值", TemplateFieldType::File, "legacy-image.png"),
    ] {
        let invalid_default = authorized_json(
            &app,
            "POST",
            "/api/inbound-templates",
            token,
            &InboundTemplateCreateRequest {
                name: name.to_owned(),
                description: None,
                fields: vec![TemplateFieldDef {
                    field_name: "字段".to_owned(),
                    field_type,
                    required: Some(false),
                    searchable: Some(false),
                    options: None,
                    default_value: Some(default_value.to_owned()),
                }],
            },
        )
        .await;
        assert_eq!(invalid_default.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(invalid_default).await, "invalid_request");
    }

    let created: InboundTemplateResponse = json_body(
        authorized_json(
            &app,
            "POST",
            "/api/inbound-templates",
            token,
            &InboundTemplateCreateRequest {
                name: "可复制收货".to_owned(),
                description: None,
                fields: vec![field("备注", TemplateFieldType::Text, false)],
            },
        )
        .await,
    )
    .await;
    let copied = authorized_json(
        &app,
        "POST",
        &format!("/api/inbound-templates/{}/copy", created.id),
        token,
        &TemplateCopyRequest {
            name: "复制收货".to_owned(),
        },
    )
    .await;
    assert_eq!(copied.status(), StatusCode::CREATED);
    let copied: InboundTemplateResponse = json_body(copied).await;
    assert_eq!(copied.fields.len(), 1);

    let duplicate = authorized_json(
        &app,
        "POST",
        "/api/inbound-templates",
        token,
        &InboundTemplateCreateRequest {
            name: "复制收货".to_owned(),
            description: None,
            fields: vec![field("备注", TemplateFieldType::Text, false)],
        },
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(duplicate).await, "template_name_taken");
}

#[tokio::test]
async fn item_template_unit_rules_are_validated_and_copied() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let created = authorized_json(
        &app,
        "POST",
        "/api/item-attribute-templates",
        &token,
        &ItemAttributeTemplateCreateRequest {
            name: "单位规则模板".to_owned(),
            description: None,
            default_inbound_template_id: None,
            fields: vec![ItemAttributeTemplateFieldDef {
                definition_id: None,
                field_name: "长度".to_owned(),
                field_type: TemplateFieldType::Number,
                required: Some(true),
                searchable: Some(true),
                catalog_visible: None,
                options: None,
                default_value: None,
                unit: Some(ItemAttributeUnitRule {
                    mode: ItemAttributeUnitMode::Select,
                    value: None,
                    options: Some(vec!["mm".to_owned(), "cm".to_owned()]),
                }),
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: ItemAttributeTemplateResponse = json_body(created).await;
    assert_eq!(created.fields[0].unit.mode, ItemAttributeUnitMode::Select);
    assert_eq!(
        created.fields[0].unit.options.as_deref(),
        Some(["mm".to_owned(), "cm".to_owned()].as_slice())
    );

    let original_definition_id = created.fields[0].field.id;
    let updated = authorized_json(
        &app,
        "PUT",
        &format!("/api/item-attribute-templates/{}", created.id),
        &token,
        &ItemAttributeTemplateUpdateRequest {
            name: None,
            description: None,
            default_inbound_template_id: None,
            fields: Some(vec![ItemAttributeTemplateFieldDef {
                definition_id: Some(original_definition_id),
                field_name: "长度规格".to_owned(),
                field_type: TemplateFieldType::Number,
                required: Some(true),
                searchable: Some(true),
                catalog_visible: None,
                options: None,
                default_value: None,
                unit: Some(ItemAttributeUnitRule {
                    mode: ItemAttributeUnitMode::Select,
                    value: None,
                    options: Some(vec!["mm".to_owned(), "cm".to_owned()]),
                }),
            }]),
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: ItemAttributeTemplateResponse = json_body(updated).await;
    assert_eq!(updated.fields[0].field.id, original_definition_id);
    assert_eq!(updated.fields[0].field.field_name, "长度规格");

    let copied = authorized_json(
        &app,
        "POST",
        &format!("/api/item-attribute-templates/{}/copy", created.id),
        &token,
        &TemplateCopyRequest {
            name: "单位规则模板副本".to_owned(),
        },
    )
    .await;
    assert_eq!(copied.status(), StatusCode::CREATED);
    let copied: ItemAttributeTemplateResponse = json_body(copied).await;
    assert_eq!(copied.fields[0].unit, updated.fields[0].unit);
    assert_ne!(copied.fields[0].field.id, original_definition_id);

    for unit in [
        ItemAttributeUnitRule {
            mode: ItemAttributeUnitMode::Fixed,
            value: None,
            options: None,
        },
        ItemAttributeUnitRule {
            mode: ItemAttributeUnitMode::Select,
            value: None,
            options: Some(vec!["kg".to_owned(), "KG".to_owned()]),
        },
        ItemAttributeUnitRule {
            mode: ItemAttributeUnitMode::None,
            value: Some("kg".to_owned()),
            options: None,
        },
    ] {
        let invalid = authorized_json(
            &app,
            "POST",
            "/api/item-attribute-templates",
            &token,
            &ItemAttributeTemplateCreateRequest {
                name: format!("无效单位规则-{:?}", unit.mode),
                description: None,
                default_inbound_template_id: None,
                fields: vec![ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "字段".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(false),
                    catalog_visible: None,
                    options: None,
                    default_value: None,
                    unit: Some(unit),
                }],
            },
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(error_code(invalid).await, "invalid_request");
    }
}

fn field(name: &str, field_type: TemplateFieldType, required: bool) -> TemplateFieldDef {
    TemplateFieldDef {
        field_name: name.to_owned(),
        field_type,
        required: Some(required),
        searchable: Some(false),
        options: None,
        default_value: None,
    }
}

#[tokio::test]
async fn item_attribute_template_limits_catalog_visible_fields_to_three() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let fields = (0..4)
        .map(|index| ItemAttributeTemplateFieldDef {
            definition_id: None,
            field_name: format!("目录字段{index}"),
            field_type: TemplateFieldType::Text,
            required: Some(false),
            searchable: Some(false),
            catalog_visible: Some(true),
            options: None,
            default_value: None,
            unit: None,
        })
        .collect();
    let response = authorized_json(
        &app,
        "POST",
        "/api/item-attribute-templates",
        &token,
        &ItemAttributeTemplateCreateRequest {
            name: "目录字段上限".to_owned(),
            description: None,
            default_inbound_template_id: None,
            fields,
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "invalid_request");
}

fn item_field(
    name: &str,
    field_type: TemplateFieldType,
    required: bool,
) -> ItemAttributeTemplateFieldDef {
    ItemAttributeTemplateFieldDef {
        definition_id: None,
        field_name: name.to_owned(),
        field_type,
        required: Some(required),
        searchable: Some(false),
        catalog_visible: None,
        options: None,
        default_value: None,
        unit: None,
    }
}

async fn authorized_json<T: serde::Serialize>(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    token: &str,
    body: &T,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(body).expect("body")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn authorized_empty(
    app: &crate::test_support::TestApp,
    method: &str,
    uri: &str,
    token: &str,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}
