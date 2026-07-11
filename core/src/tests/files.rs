//! 物品与入库属性共用图片接口和事务绑定测试。
//!
//! 本文件属于 core 集成测试层，覆盖真实 multipart、授权、删除、绑定回滚和孤儿清理。
//! 它不测试浏览器文件选择器或缩略图渲染。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::json;
use tower::ServiceExt;

use crate::{
    persistence::repository::{RbacRepository, UserRepository},
    stock::controller::{
        InboundCreateRequest, InboundItemRequest, InboundResponse, InboundTemplateCreateRequest,
        InboundTemplateResponse, ItemAttributeRequest, ItemCreateRequest, ItemResponse,
        ItemUpdateRequest, TemplateFieldDef, TemplateFieldType,
    },
    test_support::{bootstrap_location_id, error_code, json_body, login_request, seeded_app},
};

const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nwinestock";
const JPEG_BYTES: &[u8] = b"\xff\xd8\xffwinestock";
const WEBP_BYTES: &[u8] = b"RIFF\x04\x00\x00\x00WEBPwinestock";

#[tokio::test]
async fn image_upload_accepts_either_item_manage_or_inbound_create_permission() {
    let app = seeded_app().await;
    let item_manager =
        seed_user_with_permission(&app, "item-image-user", "stock.item.manage").await;
    let inbound_creator =
        seed_user_with_permission(&app, "inbound-image-user", "stock.inbound.create").await;
    crate::test_support::seed_plain_user(app.state.database(), "image-no-write", "password").await;
    let no_write = login_request(&app, "image-no-write", "password")
        .await
        .body
        .access_token;

    assert_eq!(
        upload(&app, &item_manager, "image/png", PNG_BYTES)
            .await
            .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        upload(&app, &inbound_creator, "image/png", PNG_BYTES)
            .await
            .status(),
        StatusCode::CREATED
    );
    let forbidden = upload(&app, &no_write, "image/png", PNG_BYTES).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert_eq!(error_code(forbidden).await, "permission_denied");
}

#[tokio::test]
async fn item_file_attribute_binds_to_item_and_uses_item_read_permission() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let uploaded = upload(&app, &token, "image/png", PNG_BYTES).await;
    let file_id = json_body::<serde_json::Value>(uploaded).await["id"]
        .as_i64()
        .unwrap();
    let created = authorized_json(
        &app,
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "带图片物品".to_owned(),
            sku: "ITEM-WITH-IMAGE".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &token).await,
            unit: "个".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![ItemAttributeRequest {
                template_field_id: None,
                field_name: "产品图片".to_owned(),
                field_type: TemplateFieldType::File,
                value: json!({ "file_id": file_id }),
                unit: None,
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: ItemResponse = json_body(created).await;
    assert_eq!(table_count(&app, "storage_item_file_bindings").await, 1);

    let updated = authorized_put_json(
        &app,
        &format!("/api/items/{}", created.id),
        &token,
        &ItemUpdateRequest {
            name: Some("带图片物品（已更新）".to_owned()),
            sku: None,
            category_id: None,
            attribute_template_id: None,
            image_file_id: None,
            unit: None,
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Some(vec![ItemAttributeRequest {
                template_field_id: None,
                field_name: "产品图片".to_owned(),
                field_type: TemplateFieldType::File,
                value: json!({ "file_id": file_id }),
                unit: None,
            }]),
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(table_count(&app, "storage_item_file_bindings").await, 1);
    assert_eq!(
        authorized_empty(&app, "GET", &format!("/api/files/{file_id}"), &token)
            .await
            .status(),
        StatusCode::OK
    );
    let delete = authorized_empty(&app, "DELETE", &format!("/api/files/{file_id}"), &token).await;
    assert_eq!(delete.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(delete).await, "file_already_bound");
}

#[tokio::test]
async fn item_main_image_is_required_bound_and_replaceable() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let first_image_id = crate::test_support::upload_test_image(&app, &token).await;
    let created = authorized_json(
        &app,
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "必选主图物品".to_owned(),
            sku: "REQUIRED-MAIN-IMAGE".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: first_image_id,
            unit: "个".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: ItemResponse = json_body(created).await;
    assert_eq!(created.image_file_id, first_image_id);
    assert_eq!(created.image_url, format!("/api/files/{first_image_id}"));

    let bound_delete = authorized_empty(
        &app,
        "DELETE",
        &format!("/api/files/{first_image_id}"),
        &token,
    )
    .await;
    assert_eq!(bound_delete.status(), StatusCode::CONFLICT);

    let second_image_id = crate::test_support::upload_test_image(&app, &token).await;
    let updated = authorized_put_json(
        &app,
        &format!("/api/items/{}", created.id),
        &token,
        &ItemUpdateRequest {
            name: None,
            sku: None,
            category_id: None,
            attribute_template_id: None,
            image_file_id: Some(second_image_id),
            unit: None,
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: None,
        },
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: ItemResponse = json_body(updated).await;
    assert_eq!(updated.image_file_id, second_image_id);

    let old_delete = authorized_empty(
        &app,
        "DELETE",
        &format!("/api/files/{first_image_id}"),
        &token,
    )
    .await;
    assert_eq!(old_delete.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn item_rejects_reusing_one_temporary_file_for_multiple_attributes() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let uploaded = upload(&app, &token, "image/png", PNG_BYTES).await;
    let file_id = json_body::<serde_json::Value>(uploaded).await["id"]
        .as_i64()
        .unwrap();
    let item_count_before = table_count(&app, "stock_items").await;

    let legacy_shape = authorized_json(
        &app,
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "旧文件形状".to_owned(),
            sku: "LEGACY-ITEM-FILE".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &token).await,
            unit: "个".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: vec![ItemAttributeRequest {
                template_field_id: None,
                field_name: "图片".to_owned(),
                field_type: TemplateFieldType::File,
                value: json!({ "file_id": file_id, "path": "legacy.png" }),
                unit: None,
            }],
        },
    )
    .await;
    assert_eq!(legacy_shape.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(legacy_shape).await, "invalid_request");

    let response = authorized_json(
        &app,
        "/api/items",
        &token,
        &ItemCreateRequest {
            name: "重复图片物品".to_owned(),
            sku: "DUPLICATE-ITEM-FILE".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(&app, &token).await,
            unit: "个".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: ["正面图", "背面图"]
                .into_iter()
                .map(|field_name| ItemAttributeRequest {
                    template_field_id: None,
                    field_name: field_name.to_owned(),
                    field_type: TemplateFieldType::File,
                    value: json!({ "file_id": file_id }),
                    unit: None,
                })
                .collect(),
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "invalid_request");
    assert_eq!(table_count(&app, "stock_items").await, item_count_before);
    assert_eq!(table_count(&app, "storage_item_file_bindings").await, 0);
}

#[tokio::test]
async fn image_upload_read_delete_and_validation_are_controlled() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;

    let uploaded = upload(&app, &token, "image/png", PNG_BYTES).await;
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let uploaded: serde_json::Value = json_body(uploaded).await;
    let file_id = uploaded["id"].as_i64().expect("file id should exist");
    assert_eq!(uploaded["mime_type"], "image/png");

    let read = authorized_empty(&app, "GET", &format!("/api/files/{file_id}"), &token).await;
    assert_eq!(read.status(), StatusCode::OK);
    assert_eq!(read.headers()["content-type"], "image/png");
    assert_eq!(
        to_bytes(read.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        PNG_BYTES
    );

    let forged = upload(&app, &token, "image/jpeg", PNG_BYTES).await;
    assert_eq!(forged.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(forged).await, "invalid_image_type");

    for (mime, bytes) in [("image/jpeg", JPEG_BYTES), ("image/webp", WEBP_BYTES)] {
        let response = upload(&app, &token, mime, bytes).await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let oversized = vec![0_u8; super::service::MAX_IMAGE_BYTES + 1];
    let mut oversized_png = PNG_BYTES.to_vec();
    oversized_png.extend_from_slice(&oversized[PNG_BYTES.len()..]);
    let response = upload(&app, &token, "image/png", &oversized_png).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error_code(response).await, "image_too_large");

    let deleted = authorized_empty(&app, "DELETE", &format!("/api/files/{file_id}"), &token).await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let missing = authorized_empty(&app, "GET", &format!("/api/files/{file_id}"), &token).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn inbound_file_binding_rolls_back_with_failed_order_and_blocks_temporary_delete() {
    let app = seeded_app().await;
    let admin_token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let creator_token = seed_creator(&app, "creator").await;
    let (item_id, location_id, template_id) = seed_file_item(&app, &admin_token).await;
    let uploaded = upload(&app, &creator_token, "image/png", PNG_BYTES).await;
    let file_id = json_body::<serde_json::Value>(uploaded).await["id"]
        .as_i64()
        .unwrap();

    let duplicate_file_request = InboundCreateRequest {
        submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
        source: "Supplier".to_owned(),
        notes: None,
        items: vec![
            inbound_line(item_id, location_id, template_id, file_id),
            inbound_line(item_id, location_id, template_id, file_id),
        ],
    };
    let failed = authorized_json(
        &app,
        "/api/inbound",
        &creator_token,
        &duplicate_file_request,
    )
    .await;
    assert_eq!(failed.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(failed).await, "inbound_file_unavailable");
    assert_eq!(table_count(&app, "stock_inbound_orders").await, 0);
    assert_eq!(table_count(&app, "storage_inbound_file_bindings").await, 0);

    let created = authorized_json(
        &app,
        "/api/inbound",
        &creator_token,
        &InboundCreateRequest {
            submission_mode: crate::stock::controller::InboundSubmissionMode::PendingApproval,
            source: "Supplier".to_owned(),
            notes: None,
            items: vec![inbound_line(item_id, location_id, template_id, file_id)],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: InboundResponse = json_body(created).await;
    assert_eq!(table_count(&app, "storage_inbound_file_bindings").await, 1);

    let owner_read = authorized_empty(
        &app,
        "GET",
        &format!("/api/files/{file_id}"),
        &creator_token,
    )
    .await;
    assert_eq!(owner_read.status(), StatusCode::FORBIDDEN);
    let admin_read =
        authorized_empty(&app, "GET", &format!("/api/files/{file_id}"), &admin_token).await;
    assert_eq!(admin_read.status(), StatusCode::OK);
    let delete = authorized_empty(
        &app,
        "DELETE",
        &format!("/api/files/{file_id}"),
        &creator_token,
    )
    .await;
    assert_eq!(delete.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(delete).await, "file_already_bound");

    let storage_path = file_storage_path(&app, file_id).await;
    std::fs::remove_file(app.state.storage().files_dir.join(storage_path)).unwrap();
    let approve = authorized_empty(
        &app,
        "POST",
        &format!("/api/stock-approvals/inbound/{}/approve", created.id),
        &admin_token,
    )
    .await;
    assert_eq!(approve.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(approve).await, "inbound_file_unavailable");
}

#[tokio::test]
async fn stale_unbound_image_metadata_and_content_are_cleaned() {
    let app = seeded_app().await;
    let token = login_request(&app, "admin", "password")
        .await
        .body
        .access_token;
    let uploaded = upload(&app, &token, "image/png", PNG_BYTES).await;
    let file_id = json_body::<serde_json::Value>(uploaded).await["id"]
        .as_i64()
        .unwrap();
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT storage_path FROM storage_file_objects WHERE id = ?",
            [file_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let storage_path: String = row.try_get("", "storage_path").unwrap();
    app.state
        .database()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE storage_file_objects SET created_at = '2000-01-01T00:00:00.000Z' WHERE id = ?",
            [file_id.into()],
        ))
        .await
        .unwrap();

    super::service::cleanup_orphaned_images(app.state.storage())
        .await
        .unwrap();
    assert_eq!(table_count(&app, "storage_file_objects").await, 0);
    assert!(!app.state.storage().files_dir.join(storage_path).exists());

    let interrupted_path = app
        .state
        .storage()
        .files_dir
        .join("images/ff/ff/interrupted.png");
    std::fs::create_dir_all(interrupted_path.parent().unwrap()).unwrap();
    std::fs::write(&interrupted_path, PNG_BYTES).unwrap();
    let interrupted_file = std::fs::OpenOptions::new()
        .write(true)
        .open(&interrupted_path)
        .unwrap();
    interrupted_file
        .set_times(
            std::fs::FileTimes::new().set_modified(
                std::time::SystemTime::now()
                    .checked_sub(std::time::Duration::from_secs(25 * 60 * 60))
                    .unwrap(),
            ),
        )
        .unwrap();
    super::service::cleanup_orphaned_images(app.state.storage())
        .await
        .unwrap();
    assert!(!interrupted_path.exists());
}

fn inbound_line(
    item_id: i64,
    location_id: i64,
    template_id: i64,
    file_id: i64,
) -> InboundItemRequest {
    InboundItemRequest {
        item_id,
        quantity: 1.0,
        unit_price: 2.0,
        location_id,
        batch_no: None,
        expires_at: None,
        inbound_template_id: Some(template_id),
        ext_attributes: Some(json!({ "photo": { "file_id": file_id } })),
    }
}

async fn seed_file_item(app: &crate::test_support::TestApp, token: &str) -> (i64, i64, i64) {
    let template = authorized_json(
        app,
        "/api/inbound-templates",
        token,
        &InboundTemplateCreateRequest {
            name: "File Template".to_owned(),
            description: None,
            fields: vec![TemplateFieldDef {
                field_name: "photo".to_owned(),
                field_type: TemplateFieldType::File,
                required: Some(true),
                searchable: Some(false),
                options: None,
                default_value: None,
            }],
        },
    )
    .await;
    let template: InboundTemplateResponse = json_body(template).await;
    let item = authorized_json(
        app,
        "/api/items",
        token,
        &ItemCreateRequest {
            name: "File Item".to_owned(),
            sku: "FILE-ITEM".to_owned(),
            category_id: None,
            attribute_template_id: None,
            image_file_id: crate::test_support::upload_test_image(app, token).await,
            unit: "pcs".to_owned(),
            description: None,
            default_price: None,
            reorder_point: None,
            attributes: Vec::new(),
        },
    )
    .await;
    let item: ItemResponse = json_body(item).await;
    (item.id, bootstrap_location_id(app).await, template.id)
}

async fn seed_creator(app: &crate::test_support::TestApp, username: &str) -> String {
    seed_user_with_permission(app, username, "stock.inbound.create").await
}

async fn seed_user_with_permission(
    app: &crate::test_support::TestApp,
    username: &str,
    permission: &str,
) -> String {
    crate::test_support::seed_plain_user(app.state.database(), username, "password").await;
    let users = UserRepository::new(app.state.database());
    let user = users.find_by_username(username).await.unwrap().unwrap();
    let rbac = RbacRepository::new(app.state.database());
    let permission_id = rbac.ensure_permission(permission, None).await.unwrap();
    rbac.assign_permission_to_user(user.id, permission_id)
        .await
        .unwrap();
    login_request(app, username, "password")
        .await
        .body
        .access_token
}

async fn upload(
    app: &crate::test_support::TestApp,
    token: &str,
    mime: &str,
    bytes: &[u8],
) -> axum::response::Response {
    let boundary = "winestock-test-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.png\"\r\nContent-Type: {mime}\r\n\r\n"
    ).into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/files/images")
                .header("authorization", format!("Bearer {token}"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn authorized_json<T: serde::Serialize>(
    app: &crate::test_support::TestApp,
    uri: &str,
    token: &str,
    value: &T,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(value).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn authorized_put_json<T: serde::Serialize>(
    app: &crate::test_support::TestApp,
    uri: &str,
    token: &str,
    value: &T,
) -> axum::response::Response {
    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(value).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
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
                .unwrap(),
        )
        .await
        .unwrap()
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
        .unwrap()
        .unwrap();
    row.try_get("", "count").unwrap()
}

async fn file_storage_path(app: &crate::test_support::TestApp, file_id: i64) -> String {
    let row = app
        .state
        .database()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT storage_path FROM storage_file_objects WHERE id = ?",
            [file_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get("", "storage_path").unwrap()
}
