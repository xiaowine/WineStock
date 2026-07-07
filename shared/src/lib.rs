#![forbid(unsafe_code)]

//! WineStock 平台无关配置、契约和通用类型。
//!
//! 本 crate 属于 `shared` 层，供 core、server shell 和未来平台 shell 共同使用。
//! 它只定义配置形状、平台无关枚举和 HTTP 契约，不依赖 Axum、数据库、Tauri、Android 或前端产物。

pub mod auth;
pub mod config;
pub mod error;
pub mod validation;

pub use auth::{
    AuthClientKind, AuthLoginRequest, AuthLogoutRequest, AuthRefreshRequest, AuthRegisterRequest,
    AuthTokenResponse, AuthUserResponse,
};
pub use config::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};
pub use error::ConfigParseError;

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
