//! 受控图片文件 API 模块。
//!
//! 本模块属于 core Axum 服务层，拥有物品图片属性的上传、读取、临时删除和孤儿清理。
//! 它不保存客户端路径，也不承担平台文件选择器或前端缩略图交互。

pub(crate) mod controller;
mod error;
mod service;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};

use crate::{security::AuthorizeRouteExt, state::CoreState, stock::STOCK_ITEM_MANAGE_PERMISSION};

pub use error::FileCleanupError;
pub(crate) use service::{cleanup_orphaned_images, stored_image_matches_metadata};

/// multipart 请求体上限略高于单文件上限，用于容纳边界和请求头开销。
const IMAGE_UPLOAD_BODY_LIMIT: usize = service::MAX_IMAGE_BYTES + 256 * 1024;

/// 注册图片文件 API；上传需要物品管理权限，读取和删除再按文件状态授权。
pub(crate) fn router(state: CoreState) -> Router<CoreState> {
    Router::new()
        .route(
            "/api/files/images",
            post(controller::upload_image)
                .layer(DefaultBodyLimit::max(IMAGE_UPLOAD_BODY_LIMIT))
                .require_permission(state.clone(), STOCK_ITEM_MANAGE_PERMISSION),
        )
        .route(
            "/api/files/{id}",
            get(controller::read_file)
                .merge(delete(controller::delete_file))
                .require_authenticated(state),
        )
}

#[cfg(test)]
#[path = "../tests/files.rs"]
mod tests;
