//! 图片文件 HTTP DTO 和 handler。
//!
//! 本模块属于 core 文件 HTTP 控制器层，负责 multipart 解析、受控文件响应和状态码。
//! 它不直接拼接数据库查询，也不决定前端文件选择器行为。

use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{security::CurrentUser, state::CoreState};

use super::{error::FileApiError, service};

/// 图片上传成功后的稳定文件引用信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub(crate) struct ImageFileResponse {
    /// 服务端文件对象 ID，模板 file 属性只保存该 ID。
    pub id: i64,
    /// 上传时的原始文件名，仅用于界面展示。
    pub name: String,
    /// 服务端按文件签名确认的 MIME 类型。
    pub mime_type: String,
    /// 文件大小，单位字节。
    pub size_bytes: i64,
    /// 受控读取地址，需要携带当前 access token。
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/api/files/images",
    tag = "files",
    request_body(content = String, content_type = "multipart/form-data"),
    security(("bearerAuth" = [])),
    responses(
        (status = 201, description = "Image uploaded", body = ImageFileResponse),
        (status = 400, description = "Invalid image content", body = crate::http::ApiErrorResponse),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Item manage or inbound create permission required", body = crate::http::ApiErrorResponse),
        (status = 413, description = "Image exceeds 15MB", body = crate::http::ApiErrorResponse)
    )
)]
/// 接收单张物品图片并返回稳定文件引用。
pub(crate) async fn upload_image(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImageFileResponse>), FileApiError> {
    let mut upload = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| FileApiError::InvalidUpload)?
    {
        if field.name() != Some("file") {
            continue;
        }
        if upload.is_some() {
            return Err(FileApiError::InvalidUpload);
        }
        let original_name = field.file_name().unwrap_or("image").to_owned();
        let declared_mime = field.content_type().map(str::to_owned);
        let bytes = field
            .bytes()
            .await
            .map_err(|_| FileApiError::InvalidUpload)?;
        upload = Some((original_name, declared_mime, bytes));
    }
    let (original_name, declared_mime, bytes) = upload.ok_or(FileApiError::InvalidUpload)?;
    let response = service::upload_image(
        &state,
        &current_user,
        original_name,
        declared_mime,
        bytes.as_ref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/files/{id}",
    tag = "files",
    params(("id" = i64, Path, description = "File object ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Image content", content_type = "application/octet-stream"),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "File access denied", body = crate::http::ApiErrorResponse),
        (status = 404, description = "File not found", body = crate::http::ApiErrorResponse)
    )
)]
/// 按所有权、物品读取权限或入库读取/审批权限返回受控图片内容。
pub(crate) async fn read_file(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<Response, FileApiError> {
    let content = service::read_file(&state, &current_user, id).await?;
    let mut response = Response::new(Body::from(content.bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content.mime_type).map_err(|_| FileApiError::InvalidImageType)?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=300"),
    );
    Ok(response)
}

#[utoipa::path(
    delete,
    path = "/api/files/{id}",
    tag = "files",
    params(("id" = i64, Path, description = "File object ID")),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Unbound image deleted"),
        (status = 401, description = "Invalid access token", body = crate::http::ApiErrorResponse),
        (status = 403, description = "Only the owner may delete", body = crate::http::ApiErrorResponse),
        (status = 404, description = "File not found", body = crate::http::ApiErrorResponse),
        (status = 409, description = "Bound files cannot be deleted", body = crate::http::ApiErrorResponse)
    )
)]
/// 删除当前用户拥有且尚未绑定物品的临时图片。
pub(crate) async fn delete_file(
    State(state): State<CoreState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Result<StatusCode, FileApiError> {
    service::delete_file(&state, &current_user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
