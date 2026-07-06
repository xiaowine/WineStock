//! server shell 的错误类型。
//!
//! 本模块只描述无头服务端壳自己的启动、配置、目录准备和 core 调用错误。
//! core 内部的绑定、迁移和鉴权错误通过 source 链保留，不在这里重新解释业务细节。

use std::{error::Error, fmt, io, path::PathBuf};

use winestock_core::{CoreBootstrapError, ServerStartError};
use winestock_shared::RuntimeMode;

/// 服务端 shell 自身的配置、生命周期和启动错误。
#[derive(Debug)]
pub enum ServerShellError {
    /// 获取当前可执行文件路径失败。
    ResolveExecutablePath {
        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 当前可执行文件路径没有父目录，无法定位固定配置目录。
    MissingExecutableDirectory {
        /// 无法解析父目录的可执行文件路径。
        path: PathBuf,
    },

    /// 读取已有配置文件失败。
    ReadConfig {
        /// 配置文件路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 配置文件内容不是合法启动配置。
    ParseConfig {
        /// 配置文件路径。
        path: PathBuf,

        /// JSON 解析错误。
        source: serde_json::Error,
    },

    /// 创建默认配置目录失败。
    CreateConfigDirectory {
        /// 需要创建的目录路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 序列化默认配置失败。
    SerializeDefaultConfig {
        /// 目标配置文件路径。
        path: PathBuf,

        /// JSON 序列化错误。
        source: serde_json::Error,
    },

    /// 写入默认配置文件失败。
    WriteDefaultConfig {
        /// 目标配置文件路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 配置关闭了自动启动，本服务端 shell 不继续启动。
    AutoStartDisabled,

    /// 服务端 shell 不支持远端-only 运行模式。
    UnsupportedRuntimeMode(RuntimeMode),

    /// core 没有返回本地服务启动依赖。
    LocalServiceNotInitialized,

    /// 准备数据库目录或文件目录失败。
    PrepareStorage {
        /// 创建失败的路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// core 初始化失败。
    CoreBootstrap(CoreBootstrapError),

    /// Axum 绑定或运行失败。
    Start(ServerStartError),
}

impl fmt::Display for ServerShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResolveExecutablePath { .. } => write!(f, "获取当前程序路径失败"),
            Self::MissingExecutableDirectory { path } => {
                write!(f, "当前程序路径没有可用目录: {}", path.display())
            }
            Self::ReadConfig { path, .. } => write!(f, "读取配置文件失败: {}", path.display()),
            Self::ParseConfig { path, .. } => write!(f, "解析配置文件失败: {}", path.display()),
            Self::CreateConfigDirectory { path, .. } => {
                write!(f, "创建默认配置目录失败: {}", path.display())
            }
            Self::SerializeDefaultConfig { path, .. } => {
                write!(f, "生成默认配置内容失败: {}", path.display())
            }
            Self::WriteDefaultConfig { path, .. } => {
                write!(f, "写入默认配置文件失败: {}", path.display())
            }
            Self::AutoStartDisabled => write!(
                f,
                "server.auto_start_server 为 false，服务端 shell 不会自动启动服务"
            ),
            Self::UnsupportedRuntimeMode(mode) => {
                write!(f, "服务端 shell 不支持远端客户端模式: {mode:?}")
            }
            Self::LocalServiceNotInitialized => write!(f, "core 未初始化本地服务依赖"),
            Self::PrepareStorage { path, .. } => {
                write!(f, "创建存储目录失败: {}", path.display())
            }
            Self::CoreBootstrap(source) => write!(f, "core 初始化失败: {source}"),
            Self::Start(source) => write!(f, "启动 Axum 服务失败: {source}"),
        }
    }
}

impl Error for ServerShellError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResolveExecutablePath { source } => Some(source),
            Self::ReadConfig { source, .. } => Some(source),
            Self::ParseConfig { source, .. } => Some(source),
            Self::CreateConfigDirectory { source, .. } => Some(source),
            Self::SerializeDefaultConfig { source, .. } => Some(source),
            Self::WriteDefaultConfig { source, .. } => Some(source),
            Self::PrepareStorage { source, .. } => Some(source),
            Self::CoreBootstrap(source) => Some(source),
            Self::Start(source) => Some(source),
            Self::MissingExecutableDirectory { .. }
            | Self::AutoStartDisabled
            | Self::UnsupportedRuntimeMode(_)
            | Self::LocalServiceNotInitialized => None,
        }
    }
}
