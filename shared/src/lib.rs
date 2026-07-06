#![forbid(unsafe_code)]

//! WineStock 平台无关配置、契约和通用类型。
//!
//! 本 crate 属于 `shared` 层，供 core、server shell 和未来平台 shell 共同使用。
//! 它只定义配置形状和平台无关枚举，不依赖 Axum、数据库、Tauri、Android 或前端产物。

use serde::{Deserialize, Serialize};

/// 用户名密码登录请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthLoginRequest {
    /// 登录用户名。
    pub username: String,

    /// 明文密码只允许出现在登录请求中，服务端不得持久化该值。
    pub password: String,

    /// 可选设备名称，用于标识 refresh token 来源。
    pub device_name: Option<String>,

    /// 可选客户端类型，例如 desktop、android 或 remote。
    pub client_kind: Option<String>,
}

/// 注册用户请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthRegisterRequest {
    /// 登录用户名，服务端会去除首尾空白并要求非空。
    pub username: String,

    /// 明文密码只允许出现在注册请求中，服务端会保存 Argon2 哈希。
    pub password: String,
}

/// 刷新访问令牌请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthRefreshRequest {
    /// 客户端持有的 opaque refresh token 明文；服务端只保存其哈希。
    pub refresh_token: String,
}

/// 登出请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthLogoutRequest {
    /// 需要吊销的 opaque refresh token 明文。
    pub refresh_token: String,
}

/// 鉴权接口返回的用户摘要。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuthUserResponse {
    /// 用户 ID，作为字符串返回，避免前端运行时整数精度差异。
    pub id: String,

    /// 登录用户名。
    pub username: String,

    /// 用户角色代码列表。
    pub roles: Vec<String>,

    /// 用户权限代码列表。
    pub permissions: Vec<String>,
}

/// 登录和刷新接口返回的 token 包。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuthTokenResponse {
    /// JWT access token，用于 `Authorization: Bearer` 请求头。
    pub access_token: String,

    /// opaque refresh token 明文，只在本响应中返回一次。
    pub refresh_token: String,

    /// access token 剩余有效期，单位秒。
    pub expires_in: u64,

    /// 当前登录用户摘要。
    pub user: AuthUserResponse,
}

/// WineStock v1 启动配置，只包含服务启动和本地存储两类信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// 服务运行、绑定和远端访问配置。
    pub server: ServerConfig,

    /// 本地数据库和文件目录配置。
    pub storage: StorageConfig,
}

impl AppConfig {
    /// 从 JSON 文本解析启动配置，平台壳负责决定文件位置和路径解析。
    pub fn from_json_str(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// 运行模式，决定是否启动本地服务或连接远端。
    pub mode: RuntimeMode,

    /// Axum 监听地址；`0.0.0.0` 只能作为绑定地址，不能作为访问 URL。
    pub bind_host: String,

    /// Axum 监听端口，由平台壳和 core 共同使用。
    pub port: u16,

    /// 平台壳启动时是否自动启动本地服务。
    pub auto_start_server: bool,

    /// 远端服务基础 URL，仅远端客户端模式使用。
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    /// SQLite 主数据库文件路径，平台壳负责解析为可用路径。
    pub database_path: String,

    /// 大对象文件目录，SQLite 只保存文件元数据。
    pub files_dir: String,

    /// 是否在 core 初始化时自动执行数据库迁移。
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

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
