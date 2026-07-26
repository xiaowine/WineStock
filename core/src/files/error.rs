//! 图片文件业务错误。
//!
//! 本模块属于 core 文件服务层，负责稳定错误码和启动清理错误封装。
//! 它不读取请求体或执行数据库查询。

use std::{error::Error, fmt, io};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;

use crate::http::api_error_response;

/// 图片接口稳定业务错误。
#[derive(Debug)]
pub(crate) enum FileApiError {
    /// multipart 缺少文件或结构无效。
    InvalidUpload,
    /// 声明 MIME 与文件签名不匹配，或格式不在允许列表中。
    InvalidImageType,
    /// 图片超过 15MB 限制。
    ImageTooLarge,
    /// 文件对象不存在。
    NotFound,
    /// 当前用户无权读取或删除该文件。
    PermissionDenied,
    /// 文件已经绑定物品，不能通过临时删除接口删除。
    AlreadyBound,
    /// 文件系统读写失败。
    Storage(io::Error),
    /// 文件元数据数据库读写失败。
    Database(DbErr),
}

impl IntoResponse for FileApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidUpload => (
                StatusCode::BAD_REQUEST,
                "invalid_image_upload",
                "图片上传请求无效",
            ),
            Self::InvalidImageType => (
                StatusCode::BAD_REQUEST,
                "invalid_image_type",
                "仅支持真实的 PNG、JPEG 或 WebP 图片",
            ),
            Self::ImageTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "image_too_large",
                "图片大小不能超过 15MB",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "file_not_found", "文件不存在"),
            Self::PermissionDenied => {
                (StatusCode::FORBIDDEN, "permission_denied", "无权访问该文件")
            }
            Self::AlreadyBound => (
                StatusCode::CONFLICT,
                "file_already_bound",
                "文件已绑定业务属性，不能删除",
            ),
            Self::Storage(source) => {
                let _ = source;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "file_storage_error",
                    "文件存储失败",
                )
            }
            Self::Database(source) => {
                let _ = source;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_file_error",
                    "文件服务内部错误",
                )
            }
        };
        api_error_response(status, code, message)
    }
}

impl From<DbErr> for FileApiError {
    fn from(source: DbErr) -> Self {
        Self::Database(source)
    }
}

impl From<io::Error> for FileApiError {
    fn from(source: io::Error) -> Self {
        Self::Storage(source)
    }
}

/// 服务启动阶段清理超期临时图片时的错误。
#[derive(Debug)]
pub enum FileCleanupError {
    /// 清理元数据时数据库失败。
    Database(DbErr),
    /// 清理磁盘内容时文件系统失败。
    Storage(io::Error),
}

impl fmt::Display for FileCleanupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => write!(f, "failed to clean stale image metadata"),
            Self::Storage(_) => write!(f, "failed to clean stale image content"),
        }
    }
}

impl Error for FileCleanupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(source) => Some(source),
            Self::Storage(source) => Some(source),
        }
    }
}
