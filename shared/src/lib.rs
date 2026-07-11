#![forbid(unsafe_code)]

//! WineStock 平台无关运行配置类型。
//!
//! 本 crate 属于 `shared` 层，供 core、server shell 和未来平台 shell 共同使用。
//! 它只提供运行配置、平台无关配置文件加载和基础文本校验，不依赖 Axum、数据库、Tauri、Android 或前端产物。

pub mod config;
pub mod config_file;
mod config_validation;
pub mod error;
pub mod text_validation;

pub use config::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};
pub use config_file::{load_or_create_json_config, LoadedJsonConfig};
pub use error::{ConfigFileError, ConfigParseError};

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
