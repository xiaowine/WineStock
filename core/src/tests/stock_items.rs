//! stock 模块物品接口测试。

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tower::ServiceExt;

use crate::{
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, InboundTemplateCreateRequest,
        InboundTemplateResponse, ItemAttributeRequest, ItemAttributeTemplateCreateRequest,
        ItemAttributeTemplateFieldDef, ItemAttributeTemplateResponse, ItemAttributeUnitMode,
        ItemAttributeUnitRule, ItemBatchPageResponse, ItemCreateRequest, ItemEditorResponse,
        ItemInventoryResponse, ItemMutationResponse, ItemOptionPageResponse, ItemUpdateRequest,
        TemplateFieldDef, TemplateFieldType,
    },
    test_support::{
        error_code, json_body, json_request, login_request, seed_stock_location, seeded_app,
    },
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

    let legacy_read_token =
        seed_user_with_permissions_and_login(&app, "legacy-viewer", &["stock.read"]).await;
    let legacy_read = authorized_empty_request(&app, "GET", "/api/items", &legacy_read_token).await;
    assert_eq!(legacy_read.status(), StatusCode::FORBIDDEN);

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "  Cabernet Cork  ".to_owned(),
            sku: " CORK-001 ".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &login.body.access_token)
                .await,
            unit: "pcs".to_owned(),
            description: Some("Bottle closure".to_owned()),
            default_price: Some(1.25),
            reorder_point: Some(10.0),
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let mutation: ItemMutationResponse = json_body(created).await;
    let item = get_item_editor(&app, &login.body.access_token, mutation.id).await;
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
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &login.body.access_token)
                .await,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(duplicate).await, "sku_taken");

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

    let attribute_template_id = seed_item_search_template(&app, &login.body.access_token).await;
    let updated = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/items/{}", item.id),
        &login.body.access_token,
        &ItemUpdateRequest {
            name: Some("Reserve Cork".to_owned()),
            sku: Some("CORK-002".to_owned()),
            category_id: None,
            attribute_template_id: Some(Some(attribute_template_id)),
            image_file_id: None,
            unit: None,
            description: Some(Some("Updated closure".to_owned())),
            default_price: Some(Some(1.50)),
            reorder_point: Some(Some(12.0)),
            attributes: Some(Vec::new()),
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let mutation: ItemMutationResponse = json_body(updated).await;
    let updated = get_item_editor(&app, &login.body.access_token, mutation.id).await;
    assert_eq!(updated.name, "Reserve Cork");
    assert_eq!(updated.sku, "CORK-002");
    assert_eq!(updated.attribute_template_id, Some(attribute_template_id));

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

    let audit_events = audit_events_for_entity(&app, "item", item.id).await;
    assert_eq!(audit_events.len(), 3);
    assert_eq!(audit_events[0].action, "created");
    assert_eq!(audit_events[1].action, "updated");
    assert_eq!(audit_events[2].action, "deleted");
    assert_eq!(
        audit_events[1].details["changed_fields"],
        serde_json::json!([
            "name",
            "sku",
            "attribute_template_id",
            "description",
            "default_price",
            "reorder_point"
        ])
    );
    assert_eq!(
        audit_events[1].details["new"]["attribute_template_id"],
        attribute_template_id
    );
}

#[tokio::test]
async fn item_options_expose_recommended_inbound_template_state() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let inbound_template: InboundTemplateResponse = json_body(
        authorized_json_request(
            &app,
            "POST",
            "/api/inbound-templates",
            &token,
            &InboundTemplateCreateRequest {
                name: "选品推荐入库模板".to_owned(),
                description: None,
                fields: vec![TemplateFieldDef {
                    field_name: "包装状态".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(false),
                    options: None,
                    default_value: None,
                }],
            },
        )
        .await,
    )
    .await;
    let item_template: ItemAttributeTemplateResponse = json_body(
        authorized_json_request(
            &app,
            "POST",
            "/api/item-attribute-templates",
            &token,
            &ItemAttributeTemplateCreateRequest {
                name: "选品推荐物品模板".to_owned(),
                description: None,
                default_inbound_template_id: Some(inbound_template.id),
                fields: vec![ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "规格".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(false),
                    catalog_visible: Some(false),
                    options: None,
                    default_value: None,
                    unit: None,
                }],
            },
        )
        .await,
    )
    .await;
    let item: ItemMutationResponse = json_body(
        authorized_json_request(
            &app,
            "POST",
            "/api/items",
            &token,
            &ItemCreateRequest {
                name: "推荐模板选品".to_owned(),
                sku: "RECOMMENDED-INBOUND-OPTION".to_owned(),
                category_id: None,
                attribute_template_id: Some(item_template.id),
                image_file_id: crate::test_support::upload_test_image(&app, &token).await,
                unit: "件".to_owned(),
                description: None,
                default_price: None,
                reorder_point: None,
                attributes: Vec::new(),
            },
        )
        .await,
    )
    .await;

    let options: ItemOptionPageResponse = json_body(
        authorized_empty_request(
            &app,
            "GET",
            "/api/items/options?search=RECOMMENDED-INBOUND-OPTION",
            &token,
        )
        .await,
    )
    .await;
    let option = options
        .items
        .iter()
        .find(|option| option.id == item.id)
        .unwrap();
    assert_eq!(
        option.recommended_inbound_template_id,
        Some(inbound_template.id)
    );
    assert!(option.recommended_inbound_template_available);

    let deleted = authorized_empty_request(
        &app,
        "DELETE",
        &format!("/api/inbound-templates/{}", inbound_template.id),
        &token,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let options: ItemOptionPageResponse = json_body(
        authorized_empty_request(
            &app,
            "GET",
            "/api/items/options?search=RECOMMENDED-INBOUND-OPTION",
            &token,
        )
        .await,
    )
    .await;
    let option = options
        .items
        .iter()
        .find(|option| option.id == item.id)
        .unwrap();
    assert_eq!(
        option.recommended_inbound_template_id,
        Some(inbound_template.id)
    );
    assert!(!option.recommended_inbound_template_available);
}

#[tokio::test]
async fn item_update_distinguishes_omitted_fields_from_explicit_null() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let category = authorized_json_request(
        &app,
        "POST",
        "/api/item-categories",
        &token,
        &serde_json::json!({ "name": "可清空分类", "description": null, "sort_order": 9 }),
    )
    .await;
    assert_eq!(category.status(), StatusCode::CREATED);
    let category: serde_json::Value = json_body(category).await;
    let template = authorized_json_request(
        &app,
        "POST",
        "/api/item-attribute-templates",
        &token,
        &serde_json::json!({
            "name": "可清空物品模板",
            "description": null,
            "default_inbound_template_id": null,
            "fields": [{
                "field_name": "可选属性",
                "field_type": "text",
                "required": false,
                "searchable": false,
                "options": null,
                "default_value": null
            }]
        }),
    )
    .await;
    assert_eq!(template.status(), StatusCode::CREATED);
    let template: serde_json::Value = json_body(template).await;
    let created: ItemMutationResponse = json_body(
        authorized_json_request(
            &app,
            "POST",
            "/api/items",
            &token,
            &serde_json::json!({
                "name": "可清空物品",
                "sku": "CLEARABLE-ITEM",
                "category_id": category["id"],
                "attribute_template_id": template["id"],
                "image_file_id": crate::test_support::upload_test_image(&app, &token).await,
                "unit": "个",
                "description": "待清空",
                "default_price": 12.5,
                "reorder_point": 3.0,
                "attributes": []
            }),
        )
        .await,
    )
    .await;

    let cleared = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/items/{}", created.id),
        &token,
        &serde_json::json!({
            "category_id": null,
            "attribute_template_id": null,
            "description": null,
            "default_price": null,
            "reorder_point": null,
            "attributes": []
        }),
    )
    .await;
    assert_eq!(cleared.status(), StatusCode::OK);
    let mutation: ItemMutationResponse = json_body(cleared).await;
    let cleared = get_item_editor(&app, &token, mutation.id).await;
    assert_eq!(cleared.category_id, None);
    assert_eq!(cleared.attribute_template_id, None);
    assert_eq!(cleared.description, None);
    assert_eq!(cleared.default_price, None);
    assert_eq!(cleared.reorder_point, None);

    let renamed = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/items/{}", created.id),
        &token,
        &serde_json::json!({ "name": "清空后改名" }),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let mutation: ItemMutationResponse = json_body(renamed).await;
    let renamed = get_item_editor(&app, &token, mutation.id).await;
    assert_eq!(renamed.name, "清空后改名");
    assert_eq!(renamed.category_id, None);
    assert_eq!(renamed.attribute_template_id, None);
}

#[tokio::test]
async fn item_can_select_template_while_retaining_owned_custom_attributes() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let template = authorized_json_request(
        &app,
        "POST",
        "/api/item-attribute-templates",
        &token,
        &serde_json::json!({
            "name": "混合属性模板",
            "description": null,
            "default_inbound_template_id": null,
            "fields": [{
                "field_name": "材质",
                "field_type": "select",
                "required": true,
                "searchable": true,
                "options": ["PLA", "ASA"],
                "default_value": null
            }]
        }),
    )
    .await;
    assert_eq!(template.status(), StatusCode::CREATED);
    let template: serde_json::Value = json_body(template).await;
    let created: ItemMutationResponse = json_body(
        authorized_json_request(
            &app,
            "POST",
            "/api/items",
            &token,
            &serde_json::json!({
                "name": "混合属性物品",
                "sku": "MIXED-ATTR-001",
                "image_file_id": crate::test_support::upload_test_image(&app, &token).await,
                "unit": "个",
                "attributes": [{
                    "field_name": "内部编号",
                    "field_type": "text",
                    "value": "A-01"
                }]
            }),
        )
        .await,
    )
    .await;
    let current = get_item_editor(&app, &token, created.id).await;
    let custom_definition_id = current.attributes[0].definition_id;

    let updated = authorized_json_request(
        &app,
        "PUT",
        &format!("/api/items/{}", created.id),
        &token,
        &serde_json::json!({
            "attribute_template_id": template["id"],
            "attributes": [
                {
                    "definition_id": custom_definition_id,
                    "field_name": "内部编号",
                    "field_type": "text",
                    "value": "A-01"
                },
                {
                    "definition_id": template["fields"][0]["id"],
                    "field_name": "材质",
                    "field_type": "select",
                    "value": "ASA"
                }
            ]
        }),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = get_item_editor(&app, &token, created.id).await;
    assert_eq!(updated.attribute_template_id, template["id"].as_i64());
    assert_eq!(updated.attributes.len(), 2);
    assert!(updated.attributes.iter().any(|attribute| attribute.custom));
    assert!(updated
        .attributes
        .iter()
        .any(|attribute| attribute.field_name == "材质"));
}

#[tokio::test]
async fn item_detail_returns_current_inventory_summary() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let template_id = seed_item_search_template(&app, &login.body.access_token).await;
    let item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Detail Sensor",
        "DETAIL-001",
        "DetailNeedle",
        "PrivateNeedle",
    )
    .await;

    let empty_detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}/inventory"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(empty_detail.status(), StatusCode::OK);
    let empty_detail: ItemInventoryResponse = json_body(empty_detail).await;
    assert_eq!(empty_detail.current_quantity, 0.0);
    assert_eq!(empty_detail.inventory_value, 0.0);
    assert!(empty_detail.locations.is_empty());
    assert_eq!(empty_detail.batch_count, 0);

    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "A-01",
        "DETAIL-BATCH-001",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        item_id,
        "B-02",
        "DETAIL-BATCH-002",
    )
    .await;

    let detail = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}/inventory"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail: ItemInventoryResponse = json_body(detail).await;
    assert_eq!(detail.id, item_id);
    assert_eq!(detail.current_quantity, 20.0);
    assert_eq!(detail.inventory_value, 50.0);
    assert_eq!(detail.locations.len(), 2);
    assert_eq!(detail.locations[0].location_name, "A-01");
    assert_eq!(detail.locations[0].quantity, 10.0);
    assert_eq!(detail.locations[0].value, 25.0);
    assert_eq!(detail.locations[0].batch_count, 1);
    assert_eq!(detail.locations[1].location_name, "B-02");
    assert_eq!(detail.batch_count, 2);
    let batches = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/{item_id}/batches?page=1&page_size=20"),
        &login.body.access_token,
    )
    .await;
    let batches: ItemBatchPageResponse = json_body(batches).await;
    assert_eq!(batches.total, 2);
    assert_eq!(batches.items[0].batch_no, "DETAIL-BATCH-001");
    assert_eq!(batches.items[0].remaining_quantity, 10.0);
    assert_eq!(batches.items[0].unit_cost, 2.5);
    assert_eq!(batches.items[0].value, 25.0);
    assert_eq!(batches.items[1].batch_no, "DETAIL-BATCH-002");
}

#[tokio::test]
async fn item_catalog_uses_one_stock_rule_for_counts_filtering_and_priority() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let out_of_stock = create_simple_item(&app, &token, "A 缺货", "CATALOG-OUT", Some(5.0)).await;
    let reorder_due = create_simple_item(&app, &token, "B 待补货", "CATALOG-DUE", Some(10.0)).await;
    let needs_configuration =
        create_simple_item(&app, &token, "C 需配置", "CATALOG-CONFIG", None).await;
    let normal = create_simple_item(&app, &token, "D 正常", "CATALOG-NORMAL", Some(5.0)).await;
    create_and_approve_inbound(&app, &token, reorder_due, "CAT-A", "CAT-DUE-BATCH").await;
    create_and_approve_inbound(
        &app,
        &token,
        needs_configuration,
        "CAT-B",
        "CAT-CONFIG-BATCH",
    )
    .await;
    create_and_approve_inbound(&app, &token, normal, "CAT-C", "CAT-NORMAL-BATCH").await;

    let response = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&sort=replenishment_priority",
        &token,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = json_body(response).await;
    assert_eq!(body["counts"]["total"], 4);
    assert_eq!(body["counts"]["needs_attention"], 2);
    assert_eq!(body["counts"]["out_of_stock"], 1);
    assert_eq!(body["counts"]["reorder_due"], 1);
    assert_eq!(body["counts"]["needs_configuration"], 1);
    assert_eq!(body["items"][0]["id"], out_of_stock);
    assert_eq!(body["items"][0]["stock_state"], "out_of_stock");
    assert_eq!(body["items"][1]["id"], reorder_due);
    assert_eq!(body["items"][2]["id"], needs_configuration);
    assert_eq!(body["items"][3]["id"], normal);

    let filtered = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&stock_filter=needs_attention",
        &token,
    )
    .await;
    let filtered: serde_json::Value = json_body(filtered).await;
    assert_eq!(filtered["total"], 2);
    assert_eq!(filtered["counts"]["total"], 4);
    assert_eq!(filtered["items"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn item_validation_and_authorization_fail_before_write() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;

    let missing_image = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &serde_json::json!({
            "name": "Missing Image",
            "sku": "MISSING-IMAGE",
            "unit": "pcs",
            "attributes": []
        }),
    )
    .await;
    assert_eq!(missing_image.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(missing_image).await, "invalid_request");

    let invalid = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "Bad".to_owned(),
            sku: "BAD-001".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &login.body.access_token)
                .await,
            unit: "pcs".to_owned(),
            description: None,
            default_price: Some(-1.0),
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid).await, "invalid_request");

    let invalid_date = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &login.body.access_token,
        &ItemCreateRequest {
            name: "Bad Date".to_owned(),
            sku: "BAD-DATE-001".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &login.body.access_token)
                .await,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![ItemAttributeRequest {
                definition_id: None,
                options: None,
                unit_mode: None,
                fixed_unit: None,
                unit_options: None,
                field_name: "生产日期".to_owned(),
                field_type: TemplateFieldType::Date,
                value: serde_json::json!("2026-02-31"),
                unit: None,
            }],
        },
    )
    .await;
    assert_eq!(invalid_date.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid_date).await, "invalid_request");

    let invalid_path = authorized_empty_request(
        &app,
        "GET",
        "/api/items/not-number",
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid_path.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid_path).await, "invalid_request");

    let invalid_query =
        authorized_empty_request(&app, "GET", "/api/items?page=abc", &login.body.access_token)
            .await;
    assert_eq!(invalid_query.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid_query).await, "invalid_request");

    let forbidden_token =
        seed_user_with_permissions_and_login(&app, "viewer", &["stock.item.read"]).await;
    let forbidden = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &forbidden_token,
        &ItemCreateRequest {
            name: "Viewer Item".to_owned(),
            sku: "VIEW-001".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: 1,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn item_attributes_follow_template_unit_rules() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let template = authorized_json_request(
        &app,
        "POST",
        "/api/item-attribute-templates",
        &token,
        &ItemAttributeTemplateCreateRequest {
            name: "物品单位规则".to_owned(),
            description: None,
            default_inbound_template_id: None,
            fields: vec![
                ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "长度".to_owned(),
                    field_type: TemplateFieldType::Number,
                    required: Some(true),
                    searchable: Some(true),
                    catalog_visible: None,
                    options: None,
                    default_value: None,
                    unit: Some(ItemAttributeUnitRule {
                        mode: ItemAttributeUnitMode::Fixed,
                        value: Some("mm".to_owned()),
                        options: None,
                    }),
                },
                ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "重量".to_owned(),
                    field_type: TemplateFieldType::Number,
                    required: Some(true),
                    searchable: Some(true),
                    catalog_visible: None,
                    options: None,
                    default_value: None,
                    unit: Some(ItemAttributeUnitRule {
                        mode: ItemAttributeUnitMode::Select,
                        value: None,
                        options: Some(vec!["g".to_owned(), "kg".to_owned()]),
                    }),
                },
            ],
        },
    )
    .await;
    assert_eq!(template.status(), StatusCode::CREATED);
    let template: ItemAttributeTemplateResponse = json_body(template).await;
    let length_field_id = template.fields[0].field.id;
    let weight_field_id = template.fields[1].field.id;

    let invalid = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "无效单位物品".to_owned(),
            sku: "INVALID-UNIT-001".to_owned(),
            category_id: None,
            attribute_template_id: Some(template.id),
            image_file_id: crate::test_support::upload_test_image(&app, &token).await,
            unit: "件".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![
                ItemAttributeRequest {
                    definition_id: Some(length_field_id),
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "长度".to_owned(),
                    field_type: TemplateFieldType::Number,
                    value: serde_json::json!(120),
                    unit: Some("cm".to_owned()),
                },
                ItemAttributeRequest {
                    definition_id: Some(weight_field_id),
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "重量".to_owned(),
                    field_type: TemplateFieldType::Number,
                    value: serde_json::json!(2.5),
                    unit: Some("lb".to_owned()),
                },
            ],
        },
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(invalid).await, "invalid_request");

    let created = authorized_json_request(
        &app,
        "POST",
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "有效单位物品".to_owned(),
            sku: "VALID-UNIT-001".to_owned(),
            category_id: None,
            attribute_template_id: Some(template.id),
            image_file_id: crate::test_support::upload_test_image(&app, &token).await,
            unit: "件".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![
                ItemAttributeRequest {
                    definition_id: Some(length_field_id),
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "长度".to_owned(),
                    field_type: TemplateFieldType::Number,
                    value: serde_json::json!(120),
                    unit: None,
                },
                ItemAttributeRequest {
                    definition_id: Some(weight_field_id),
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "重量".to_owned(),
                    field_type: TemplateFieldType::Number,
                    value: serde_json::json!(2.5),
                    unit: Some("kg".to_owned()),
                },
            ],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let mutation: ItemMutationResponse = json_body(created).await;
    let created = get_item_editor(&app, &token, mutation.id).await;
    assert_eq!(created.attributes[0].unit.as_deref(), Some("mm"));
    assert_eq!(created.attributes[1].unit.as_deref(), Some("kg"));
}

#[tokio::test]
async fn item_search_and_structured_filters_share_catalog_scope() {
    let app = seeded_app().await;
    let login = login_request(&app, "admin", "password").await;
    let template_id = seed_item_search_template(&app, &login.body.access_token).await;
    let item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Searchable Sensor",
        "SEARCH-001",
        "CurrentNeedle",
        "PrivateNeedle",
    )
    .await;
    create_and_approve_inbound(&app, &login.body.access_token, item_id, "A-01", "CUR-001").await;
    create_and_approve_inbound(&app, &login.body.access_token, item_id, "A-02", "CUR-002").await;

    let historical_item_id = seed_item(
        &app,
        &login.body.access_token,
        template_id,
        "Historical Sensor",
        "HIST-001",
        "GoneNeedle",
        "HiddenGoneNeedle",
    )
    .await;
    create_and_approve_inbound(
        &app,
        &login.body.access_token,
        historical_item_id,
        "Z-99",
        "GONE-001",
    )
    .await;
    zero_item_inventory(&app, historical_item_id).await;

    let by_template_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=CurrentNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_template_value.status(), StatusCode::OK);
    let by_template_value: serde_json::Value = json_body(by_template_value).await;
    assert_eq!(by_template_value["total"], 1);
    assert_eq!(by_template_value["items"][0]["id"], item_id);
    assert_eq!(
        by_template_value["items"][0]["catalog_attributes"][0]["name"],
        "brand"
    );
    assert_eq!(
        by_template_value["items"][0]["catalog_attributes"][0]["value"],
        "CurrentNeedle"
    );

    let by_non_searchable_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=PrivateNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_non_searchable_value.status(), StatusCode::OK);
    let by_non_searchable_value: serde_json::Value = json_body(by_non_searchable_value).await;
    assert_eq!(by_non_searchable_value["total"], 1);

    let by_template_name = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=SearchFilterTemplate",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_template_name.status(), StatusCode::OK);
    let by_template_name: serde_json::Value = json_body(by_template_name).await;
    assert_eq!(by_template_name["total"], 2);

    let by_exhausted_value = authorized_empty_request(
        &app,
        "GET",
        "/api/items?page=1&page_size=20&search=GoneNeedle",
        &login.body.access_token,
    )
    .await;
    assert_eq!(by_exhausted_value.status(), StatusCode::OK);
    let by_exhausted_value: serde_json::Value = json_body(by_exhausted_value).await;
    assert_eq!(by_exhausted_value["total"], 1);

    let empty_search =
        authorized_empty_request(&app, "GET", "/api/items?search=", &login.body.access_token).await;
    assert_eq!(empty_search.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(empty_search).await, "invalid_request");

    let missing_token = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/items/filter-values")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let filter_values = authorized_empty_request(
        &app,
        "GET",
        "/api/items/filter-values",
        &login.body.access_token,
    )
    .await;
    assert_eq!(filter_values.status(), StatusCode::OK);
    let filter_values: serde_json::Value = json_body(filter_values).await;
    let brand_key = filter_field_key_by_label(&filter_values, "brand")
        .expect("searchable template field should expose stable key");
    assert!(brand_key.starts_with("template:"));
    assert_eq!(
        filter_value_count(&filter_values, &brand_key, "CurrentNeedle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, &brand_key, "GoneNeedle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:unit", "pcs"),
        Some(2)
    );
    assert_eq!(
        filter_value_count(&filter_values, "base:location", "A-01"),
        Some(1)
    );
    assert!(filter_field_key_by_label(&filter_values, "internal_note").is_none());

    let brand_filters = percent_encode_query_value(
        &serde_json::json!([{"key": brand_key, "values": ["CurrentNeedle"]}]).to_string(),
    );
    let filtered = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items?page=1&page_size=20&filters={brand_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(filtered.status(), StatusCode::OK);
    let filtered: serde_json::Value = json_body(filtered).await;
    assert_eq!(filtered["total"], 1);
    assert_eq!(filtered["counts"]["total"], 1);
    assert_eq!(filtered["items"][0]["id"], item_id);

    let or_filters = percent_encode_query_value(
        &serde_json::json!([
            {"key": brand_key, "values": ["CurrentNeedle"]},
            {"key": brand_key, "values": ["GoneNeedle"]}
        ])
        .to_string(),
    );
    let or_filtered = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items?page=1&page_size=20&filters={or_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(or_filtered.status(), StatusCode::OK);
    let or_filtered: serde_json::Value = json_body(or_filtered).await;
    assert_eq!(or_filtered["total"], 2);

    let unit_filters = percent_encode_query_value(
        &serde_json::json!([{"key": "base:unit", "values": ["pcs"]}]).to_string(),
    );
    let unit_filtered = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items?page=1&page_size=20&filters={unit_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(unit_filtered.status(), StatusCode::OK);
    let unit_filtered: serde_json::Value = json_body(unit_filtered).await;
    assert_eq!(unit_filtered["total"], 2);

    let faceted_values = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items/filter-values?filters={brand_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(faceted_values.status(), StatusCode::OK);
    let faceted_values: serde_json::Value = json_body(faceted_values).await;
    assert_eq!(
        filter_value_count(&faceted_values, &brand_key, "GoneNeedle"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&faceted_values, "base:location", "A-01"),
        Some(1)
    );
    assert_eq!(
        filter_value_count(&faceted_values, "base:location", "Z-99"),
        None
    );

    let combined_filters = percent_encode_query_value(
        &serde_json::json!([
            {"key": "base:location", "values": ["A-01"]},
            {"key": brand_key, "values": ["GoneNeedle"]}
        ])
        .to_string(),
    );
    let combined = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items?page=1&page_size=20&filters={combined_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(combined.status(), StatusCode::OK);
    let combined: serde_json::Value = json_body(combined).await;
    assert_eq!(combined["total"], 0);

    let invalid_filters = percent_encode_query_value(
        &serde_json::json!([{"key": "template:999999", "values": ["x"]}]).to_string(),
    );
    let invalid = authorized_empty_request(
        &app,
        "GET",
        &format!("/api/items?filters={invalid_filters}"),
        &login.body.access_token,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let malformed = authorized_empty_request(
        &app,
        "GET",
        "/api/items?filters=not-json",
        &login.body.access_token,
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
}

async fn seed_item_search_template(app: &crate::test_support::TestApp, access_token: &str) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/item-attribute-templates",
        access_token,
        &ItemAttributeTemplateCreateRequest {
            name: "SearchFilterTemplate".to_owned(),
            description: Some("search metadata template".to_owned()),
            default_inbound_template_id: None,
            fields: vec![
                ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "brand".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(true),
                    catalog_visible: Some(true),
                    options: None,
                    default_value: None,
                    unit: None,
                },
                ItemAttributeTemplateFieldDef {
                    definition_id: None,
                    field_name: "internal_note".to_owned(),
                    field_type: TemplateFieldType::Text,
                    required: Some(false),
                    searchable: Some(false),
                    catalog_visible: None,
                    options: None,
                    default_value: None,
                    unit: None,
                },
            ],
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let template: ItemAttributeTemplateResponse = json_body(response).await;

    template.id
}

async fn seed_item(
    app: &crate::test_support::TestApp,
    access_token: &str,
    template_id: i64,
    name: &str,
    sku: &str,
    brand: &str,
    internal_note: &str,
) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: name.to_owned(),
            sku: sku.to_owned(),
            category_id: None,
            attribute_template_id: Some(template_id),
            image_file_id: crate::test_support::upload_test_image(app, access_token).await,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![
                ItemAttributeRequest {
                    definition_id: None,
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "brand".to_owned(),
                    field_type: TemplateFieldType::Text,
                    value: serde_json::json!(brand),
                    unit: None,
                },
                ItemAttributeRequest {
                    definition_id: None,
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
                    field_name: "internal_note".to_owned(),
                    field_type: TemplateFieldType::Text,
                    value: serde_json::json!(internal_note),
                    unit: None,
                },
            ],
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let item: ItemMutationResponse = json_body(response).await;
    item.id
}

async fn get_item_editor(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
) -> ItemEditorResponse {
    let response =
        authorized_empty_request(app, "GET", &format!("/api/items/{item_id}"), access_token).await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn create_simple_item(
    app: &crate::test_support::TestApp,
    access_token: &str,
    name: &str,
    sku: &str,
    reorder_point: Option<f64>,
) -> i64 {
    let response = authorized_json_request(
        app,
        "POST",
        "/api/items",
        access_token,
        &ItemCreateRequest {
            name: name.to_owned(),
            sku: sku.to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(app, access_token).await,
            unit: "个".to_owned(),
            description: None,
            default_price: None,
            reorder_point,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json_body::<ItemMutationResponse>(response).await.id
}

async fn create_and_approve_inbound(
    app: &crate::test_support::TestApp,
    access_token: &str,
    item_id: i64,
    location: &str,
    batch_no: &str,
) -> InboundResponse {
    let location_id = seed_stock_location(app, location).await;
    let created = authorized_json_request(
        app,
        "POST",
        "/api/inbound",
        access_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "Search Supplier".to_owned(),
            notes: Some("search fixture".to_owned()),
            items: vec![InboundItemRequest {
                item_id,
                quantity: 10.0,
                unit_price: 2.5,
                location_id,
                batch_no: Some(batch_no.to_owned()),
                expires_at: Some("2028-01-01".to_owned()),
                inbound_template_id: None,
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
    json_body(approved).await
}

async fn zero_item_inventory(app: &crate::test_support::TestApp, item_id: i64) {
    app.state
        .database()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE stock_batches SET remaining_quantity = 0 WHERE item_id = ?",
            [item_id.into()],
        ))
        .await
        .expect("inventory update should succeed");
}

#[derive(Debug, Clone, PartialEq)]
struct AuditEventRow {
    action: String,
    details: serde_json::Value,
}

async fn audit_events_for_entity(
    app: &crate::test_support::TestApp,
    entity_type: &str,
    entity_id: i64,
) -> Vec<AuditEventRow> {
    app.state
        .database()
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            r#"
            SELECT action, details_json
            FROM audit_events
            WHERE entity_type = ? AND entity_id = ?
            ORDER BY id ASC
            "#,
            vec![entity_type.into(), entity_id.into()],
        ))
        .await
        .expect("audit events should query")
        .into_iter()
        .map(|row| AuditEventRow {
            action: row.try_get("", "action").expect("action should decode"),
            details: row
                .try_get::<Option<String>>("", "details_json")
                .expect("details should decode")
                .and_then(|details| serde_json::from_str(&details).ok())
                .expect("details should be json"),
        })
        .collect()
}

fn filter_value_count(payload: &serde_json::Value, key: &str, value: &str) -> Option<u64> {
    payload["fields"]
        .as_array()?
        .iter()
        .find(|field| field.get("key").and_then(serde_json::Value::as_str) == Some(key))?
        .get("values")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("value").and_then(serde_json::Value::as_str) == Some(value))?
        .get("count")?
        .as_u64()
}

fn filter_field_key_by_label(payload: &serde_json::Value, label: &str) -> Option<String> {
    payload["fields"]
        .as_array()?
        .iter()
        .find(|field| field.get("label").and_then(serde_json::Value::as_str) == Some(label))?
        .get("key")?
        .as_str()
        .map(str::to_owned)
}

fn percent_encode_query_value(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
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
