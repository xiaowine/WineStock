//! 图片文件业务服务。
//!
//! 本模块属于 core 文件服务层，负责签名校验、SHA-256 内容寻址、访问授权和孤儿清理。
//! 它不解析 multipart，也不暴露服务端绝对路径。

use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};

use sha2::{Digest, Sha256};

use crate::{
    persistence::{
        repository::{CreateFileObject, FileObjectRepository},
        StorageRuntime,
    },
    security::CurrentUser,
    state::CoreState,
    stock::{
        STOCK_INBOUND_APPROVE_PERMISSION, STOCK_INBOUND_READ_PERMISSION,
        STOCK_ITEM_MANAGE_PERMISSION, STOCK_ITEM_READ_PERMISSION,
    },
};

use super::{
    controller::ImageFileResponse,
    error::{FileApiError, FileCleanupError},
};

/// 单张模板图片的最大字节数。
pub(super) const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;
const ORPHAN_RETENTION_HOURS: i64 = 24;

/// 受控读取后的图片内容。
pub(super) struct FileContent {
    /// 已校验的图片 MIME 类型。
    pub mime_type: String,
    /// 图片二进制内容。
    pub bytes: Vec<u8>,
}

/// 校验并保存单张图片；磁盘内容按 SHA-256 派生相对路径存储。
pub(super) async fn upload_image(
    state: &CoreState,
    current_user: &CurrentUser,
    original_name: String,
    declared_mime: Option<String>,
    bytes: &[u8],
) -> Result<ImageFileResponse, FileApiError> {
    cleanup_orphaned_images(state.storage())
        .await
        .map_err(|error| match error {
            FileCleanupError::Database(source) => FileApiError::Database(source),
            FileCleanupError::Storage(source) => FileApiError::Storage(source),
        })?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(FileApiError::ImageTooLarge);
    }
    let detected = detect_image_type(bytes).ok_or(FileApiError::InvalidImageType)?;
    if declared_mime.as_deref().map(str::trim) != Some(detected.mime_type) {
        return Err(FileApiError::InvalidImageType);
    }

    let sha256 = sha256_hex(bytes);
    let relative_path = image_relative_path(&sha256, detected.extension);
    let absolute_path = resolve_storage_path(&state.storage().files_dir, &relative_path)?;
    write_content_if_missing(&absolute_path, bytes)?;

    let repository = FileObjectRepository::new(state.database());
    let file = repository
        .create_metadata(CreateFileObject {
            sha256,
            mime_type: Some(detected.mime_type.to_owned()),
            size_bytes: i64::try_from(bytes.len()).map_err(|_| FileApiError::ImageTooLarge)?,
            storage_path: relative_path,
            original_name: Some(normalize_original_name(&original_name)),
            owner_user_id: Some(current_user.user_id),
        })
        .await?;

    Ok(ImageFileResponse {
        id: file.id,
        name: file.original_name.unwrap_or_else(|| "image".to_owned()),
        mime_type: detected.mime_type.to_owned(),
        size_bytes: file.size_bytes,
        url: format!("/api/files/{}", file.id),
    })
}

/// 按临时所有权或已绑定入库权限读取图片内容。
pub(super) async fn read_file(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<FileContent, FileApiError> {
    if id < 1 {
        return Err(FileApiError::NotFound);
    }
    let repository = FileObjectRepository::new(state.database());
    let record = repository
        .find_access_record(id)
        .await?
        .ok_or(FileApiError::NotFound)?;
    let authorized = if record.inbound_order_item_id.is_some() {
        current_user.has_permission(STOCK_INBOUND_READ_PERMISSION)
            || current_user.has_permission(STOCK_INBOUND_APPROVE_PERMISSION)
    } else if record.item_id.is_some() {
        current_user.has_permission(STOCK_ITEM_READ_PERMISSION)
            || current_user.has_permission(STOCK_ITEM_MANAGE_PERMISSION)
    } else {
        record.file.owner_user_id == Some(current_user.user_id)
    };
    if !authorized {
        return Err(FileApiError::PermissionDenied);
    }
    let mime_type = record
        .file
        .mime_type
        .ok_or(FileApiError::InvalidImageType)?;
    let path = resolve_storage_path(&state.storage().files_dir, &record.file.storage_path)?;
    let bytes = fs::read(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            FileApiError::NotFound
        } else {
            FileApiError::Storage(source)
        }
    })?;
    let detected = detect_image_type(&bytes).ok_or(FileApiError::InvalidImageType)?;
    if detected.mime_type != mime_type {
        return Err(FileApiError::InvalidImageType);
    }
    Ok(FileContent { mime_type, bytes })
}

/// 复核已保存图片的磁盘内容、大小、SHA-256、MIME 和文件签名是否仍与元数据一致。
///
/// 入库创建和审批会调用本函数；任何路径异常、文件缺失或内容不一致都返回 false，
/// 调用方应将其作为业务文件不可用处理，不向客户端暴露服务端路径。
pub(crate) fn stored_image_matches_metadata(
    storage: &StorageRuntime,
    file: &crate::persistence::entity::file_object::Model,
) -> bool {
    let Some(expected_mime) = file.mime_type.as_deref() else {
        return false;
    };
    let Ok(path) = resolve_storage_path(&storage.files_dir, &file.storage_path) else {
        return false;
    };
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    if i64::try_from(bytes.len()).ok() != Some(file.size_bytes) || sha256_hex(&bytes) != file.sha256
    {
        return false;
    }
    detect_image_type(&bytes).is_some_and(|detected| detected.mime_type == expected_mime)
}

/// 删除当前用户拥有的未绑定图片，并在没有其它元数据引用时删除磁盘内容。
pub(super) async fn delete_file(
    state: &CoreState,
    current_user: &CurrentUser,
    id: i64,
) -> Result<(), FileApiError> {
    let repository = FileObjectRepository::new(state.database());
    let record = repository
        .find_access_record(id)
        .await?
        .ok_or(FileApiError::NotFound)?;
    if record.file.owner_user_id != Some(current_user.user_id) {
        return Err(FileApiError::PermissionDenied);
    }
    if record.is_bound() {
        return Err(FileApiError::AlreadyBound);
    }
    if !repository
        .delete_unbound_owned(id, current_user.user_id)
        .await?
    {
        return Err(FileApiError::AlreadyBound);
    }
    delete_content_when_unreferenced(state.storage(), &repository, &record.file.storage_path)
        .await
        .map_err(|error| match error {
            FileCleanupError::Database(source) => FileApiError::Database(source),
            FileCleanupError::Storage(source) => FileApiError::Storage(source),
        })
}

/// 清理超过 24 小时仍未绑定的临时图片元数据和无引用磁盘内容。
pub(crate) async fn cleanup_orphaned_images(
    storage: &StorageRuntime,
) -> Result<(), FileCleanupError> {
    let repository = FileObjectRepository::new(&storage.database);
    let stale = repository
        .list_stale_unbound(ORPHAN_RETENTION_HOURS)
        .await
        .map_err(FileCleanupError::Database)?;
    for file in stale {
        if repository
            .delete_stale_unbound(file.id)
            .await
            .map_err(FileCleanupError::Database)?
        {
            delete_content_when_unreferenced(storage, &repository, &file.storage_path).await?;
        }
    }
    cleanup_untracked_content(storage, &repository).await?;
    Ok(())
}

/// 清理没有任何数据库元数据引用且磁盘修改时间已超过保留期的内容文件。
///
/// 该路径覆盖进程在写入文件后、创建元数据前中断的窄窗口；只扫描受控的 `images/` 子目录。
async fn cleanup_untracked_content(
    storage: &StorageRuntime,
    repository: &FileObjectRepository<'_>,
) -> Result<(), FileCleanupError> {
    let images_root = storage.files_dir.join("images");
    let mut paths = Vec::new();
    collect_content_files(&images_root, &mut paths).map_err(FileCleanupError::Storage)?;
    let retention = Duration::from_secs((ORPHAN_RETENTION_HOURS as u64) * 60 * 60);
    for path in paths {
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .map_err(FileCleanupError::Storage)?;
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            < retention
        {
            continue;
        }
        let relative = storage_relative_path(&storage.files_dir, &path)?;
        if repository
            .count_by_storage_path(&relative)
            .await
            .map_err(FileCleanupError::Database)?
            == 0
        {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(FileCleanupError::Storage(source)),
            }
        }
    }
    Ok(())
}

fn collect_content_files(directory: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(source),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect_content_files(&path, result)?;
        } else if path.is_file() {
            result.push(path);
        }
    }
    Ok(())
}

fn storage_relative_path(root: &Path, path: &Path) -> Result<String, FileCleanupError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        FileCleanupError::Storage(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "image content escaped storage root",
        ))
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(FileCleanupError::Storage(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid image content path",
            )));
        };
        parts.push(part.to_string_lossy().into_owned());
    }
    Ok(parts.join("/"))
}

async fn delete_content_when_unreferenced(
    storage: &StorageRuntime,
    repository: &FileObjectRepository<'_>,
    relative_path: &str,
) -> Result<(), FileCleanupError> {
    let references = repository
        .count_by_storage_path(relative_path)
        .await
        .map_err(FileCleanupError::Database)?;
    if references != 0 {
        return Ok(());
    }
    let path =
        resolve_storage_path(&storage.files_dir, relative_path).map_err(|error| match error {
            FileApiError::Storage(source) => FileCleanupError::Storage(source),
            _ => FileCleanupError::Storage(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid storage path",
            )),
        })?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FileCleanupError::Storage(source)),
    }
}

struct DetectedImageType {
    mime_type: &'static str,
    extension: &'static str,
}

fn detect_image_type(bytes: &[u8]) -> Option<DetectedImageType> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(DetectedImageType {
            mime_type: "image/png",
            extension: "png",
        });
    }
    if bytes.len() >= 4 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Some(DetectedImageType {
            mime_type: "image/jpeg",
            extension: "jpg",
        });
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(DetectedImageType {
            mime_type: "image/webp",
            extension: "webp",
        });
    }
    None
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn image_relative_path(sha256: &str, extension: &str) -> String {
    format!(
        "images/{}/{}/{}.{}",
        &sha256[0..2],
        &sha256[2..4],
        sha256,
        extension
    )
}

fn resolve_storage_path(root: &Path, relative: &str) -> Result<PathBuf, FileApiError> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(FileApiError::Storage(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid relative storage path",
        )));
    }
    Ok(root.join(relative))
}

fn write_content_if_missing(path: &Path, bytes: &[u8]) -> Result<(), FileApiError> {
    if path.exists() {
        return Ok(());
    }
    let parent = path.parent().ok_or_else(|| {
        FileApiError::Storage(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "image path has no parent",
        ))
    })?;
    fs::create_dir_all(parent)?;
    match fs::write(path, bytes) {
        Ok(()) => Ok(()),
        Err(source) => Err(FileApiError::Storage(source)),
    }
}

fn normalize_original_name(value: &str) -> String {
    let name = value.trim();
    let name = if name.is_empty() { "image" } else { name };
    name.chars().take(255).collect()
}
