//! 平台无关启动配置实体。
//!
//! 本模块属于 `shared` 层，定义运行模式、网络绑定和本地存储配置。
//! 它不读取配置文件，不创建目录，也不启动 Axum。

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::{
    config_validation::validate_optional_http_url, error::ConfigParseError,
    text_validation::validate_not_blank,
};

/// WineStock v1 启动配置，只包含服务启动和本地存储两类信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, garde::Validate)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// 服务运行、绑定和远端访问配置。
    #[garde(dive)]
    pub server: ServerConfig,

    /// 本地数据库和文件目录配置。
    #[garde(dive)]
    pub storage: StorageConfig,
}

impl AppConfig {
    /// 从 JSON 文本解析启动配置，平台壳负责决定文件位置和路径解析。
    pub fn from_json_str(input: &str) -> Result<Self, ConfigParseError> {
        let config: Self = serde_json::from_str(input).map_err(ConfigParseError::Json)?;
        config.validate().map_err(ConfigParseError::Validation)?;
        Ok(config)
    }

    /// 输出稳定的 JSON 配置文本，便于平台壳创建默认配置文件。
    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

/// 服务运行模式，决定平台壳是否启动本地 Axum 或连接远端服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    /// 只连接远端服务，不启动本地 Axum，也不使用本地数据库。
    ClientOnly,

    /// 启动本地 Axum 供本机 UI 使用，默认绑定 loopback。
    SelfHosted,

    /// 启动本地 Axum 作为可被其他客户端访问的服务端。
    ServerMode,

    /// 连接远端服务；语义上保留给需要明确远端连接的客户端壳。
    ConnectToRemote,
}

impl RuntimeMode {
    /// 这些模式需要本地服务和本地数据库。
    pub fn uses_local_service(self) -> bool {
        matches!(self, Self::SelfHosted | Self::ServerMode)
    }

    /// 这些模式只作为客户端访问远端服务。
    pub fn uses_remote_service(self) -> bool {
        matches!(self, Self::ClientOnly | Self::ConnectToRemote)
    }
}

/// Axum 服务启动和访问相关配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, garde::Validate)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// 运行模式，决定是否启动本地服务或连接远端。
    #[garde(skip)]
    pub mode: RuntimeMode,

    /// Axum 监听地址；`0.0.0.0` 只能作为绑定地址，不能作为访问 URL。
    #[garde(length(min = 1, max = 255), ip)]
    pub bind_host: String,

    /// Axum 监听端口，由平台壳和 core 共同使用。
    #[garde(range(min = 1))]
    pub port: u16,

    /// 平台壳启动时是否自动启动本地服务。
    #[garde(skip)]
    pub auto_start_server: bool,

    /// 远端服务基础 URL，仅远端客户端模式使用。
    #[garde(length(max = 2048), custom(validate_optional_http_url))]
    pub remote_base_url: String,
}

impl ServerConfig {
    /// 当前运行模式是否需要本地服务。
    pub fn uses_local_service(&self) -> bool {
        self.mode.uses_local_service()
    }

    /// 平台壳启动时是否应自动启动本地服务。
    pub fn should_auto_start_local_service(&self) -> bool {
        self.uses_local_service() && self.auto_start_server
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::SelfHosted,
            bind_host: "127.0.0.1".to_owned(),
            port: 17890,
            auto_start_server: true,
            remote_base_url: String::new(),
        }
    }
}

/// 本地持久化配置，路径由平台壳补齐并传入 core。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, garde::Validate)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    /// SQLite 主数据库文件路径，平台壳负责解析为可用路径。
    #[garde(length(min = 1, max = 4096), custom(validate_not_blank))]
    pub database_path: String,

    /// 大对象文件目录，SQLite 只保存文件元数据。
    #[garde(length(min = 1, max = 4096), custom(validate_not_blank))]
    pub files_dir: String,

    /// 是否在 core 初始化时自动执行数据库迁移。
    #[garde(skip)]
    pub auto_migrate: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_path: "data/winestock.sqlite".to_owned(),
            files_dir: "data/files".to_owned(),
            auto_migrate: true,
        }
    }
}
