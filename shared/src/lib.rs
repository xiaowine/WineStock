#![forbid(unsafe_code)]

//! WineStock 平台无关运行配置类型。
//!
//! 本 crate 属于 `shared` 层，供 core、server shell 和未来平台 shell 共同使用。
//! 它只定义运行配置形状、运行模式、配置解析错误和基础文本校验，不依赖 Axum、数据库、Tauri、Android 或前端产物。

pub mod config;
mod config_validation;
pub mod error;
pub mod text_validation;

pub use config::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};
pub use error::ConfigParseError;

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
