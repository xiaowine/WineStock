//! 物品图片接口和事务绑定测试。
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
        ItemAttributeRequest, ItemCreateRequest, ItemEditorResponse, ItemMutationResponse,
        ItemUpdateRequest, TemplateFieldType,
    },
    test_support::{error_code, json_body, login_request, seeded_app},
};

const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nwinestock";
const JPEG_BYTES: &[u8] = b"\xff\xd8\xffwinestock";
const WEBP_BYTES: &[u8] = b"RIFF\x04\x00\x00\x00WEBPwinestock";

#[tokio::test]
async fn image_upload_requires_item_manage_permission() {
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
    let inbound_forbidden = upload(&app, &inbound_creator, "image/png", PNG_BYTES).await;
    assert_eq!(inbound_forbidden.status(), StatusCode::FORBIDDEN);
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
                definition_id: None,
                options: None,
                unit_mode: None,
                fixed_unit: None,
                unit_options: None,
                field_name: "产品图片".to_owned(),
                field_type: TemplateFieldType::File,
                value: json!({ "file_id": file_id }),
                unit: None,
            }],
        },
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let mutation: ItemMutationResponse = json_body(created).await;
    let created = get_item_editor(&app, &token, mutation.id).await;
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
                definition_id: None,
                options: None,
                unit_mode: None,
                fixed_unit: None,
                unit_options: None,
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
    let mutation: ItemMutationResponse = json_body(created).await;
    let created = get_item_editor(&app, &token, mutation.id).await;
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
    let mutation: ItemMutationResponse = json_body(updated).await;
    let updated = get_item_editor(&app, &token, mutation.id).await;
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
                definition_id: None,
                options: None,
                unit_mode: None,
                fixed_unit: None,
                unit_options: None,
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
                    definition_id: None,
                    options: None,
                    unit_mode: None,
                    fixed_unit: None,
                    unit_options: None,
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
        .query_one_raw(Statement::from_sql_and_values(
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
        .execute_raw(Statement::from_sql_and_values(
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

async fn get_item_editor(
    app: &crate::test_support::TestApp,
    token: &str,
    item_id: i64,
) -> ItemEditorResponse {
    let response = authorized_empty(app, "GET", &format!("/api/items/{item_id}"), token).await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
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
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) AS count FROM {table}"),
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get("", "count").unwrap()
}
