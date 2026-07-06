use std::{error::Error, fmt, io, path::PathBuf};

use winestock_core::{CoreBootstrapError, ServerStartError};
use winestock_shared::RuntimeMode;

/// 服务端 shell 自身的配置、生命周期和启动错误。
#[derive(Debug)]
pub enum ServerShellError {
    ResolveExecutablePath {
        source: io::Error,
    },
    MissingExecutableDirectory {
        path: PathBuf,
    },
    ReadConfig {
        path: PathBuf,
        source: io::Error,
    },
    ParseConfig {
        path: PathBuf,
        source: serde_json::Error,
    },
    CreateConfigDirectory {
        path: PathBuf,
        source: io::Error,
    },
    SerializeDefaultConfig {
        path: PathBuf,
        source: serde_json::Error,
    },
    WriteDefaultConfig {
        path: PathBuf,
        source: io::Error,
    },
    AutoStartDisabled,
    UnsupportedRuntimeMode(RuntimeMode),
    LocalServiceNotInitialized,
    PrepareStorage {
        path: PathBuf,
        source: io::Error,
    },
    CoreBootstrap(CoreBootstrapError),
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
