//! server shell 的错误类型。
//!
//! 本模块只描述无头服务端壳自己的启动、配置、目录准备和 core 调用错误。
//! core 内部的绑定、迁移和鉴权错误通过 source 链保留，不在这里重新解释业务细节。

use std::{error::Error, fmt, io, path::PathBuf};

use winestock_core::LocalServiceRuntimeError;
use winestock_shared::{ConfigFileError, RuntimeMode};

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

    /// shared 层读取、解析或创建 JSON 配置文件失败。
    LoadConfigFile(ConfigFileError),

    /// 配置关闭了自动启动，本服务端 shell 不继续启动。
    AutoStartDisabled,

    /// 服务端 shell 不支持远端-only 运行模式。
    UnsupportedRuntimeMode(RuntimeMode),

    /// 准备数据库目录或文件目录失败。
    PrepareStorage {
        /// 创建失败的路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// core 本地服务启动、运行或关闭失败。
    LocalService(LocalServiceRuntimeError),
}

impl fmt::Display for ServerShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResolveExecutablePath { .. } => write!(f, "获取当前程序路径失败"),
            Self::MissingExecutableDirectory { path } => {
                write!(f, "当前程序路径没有可用目录: {}", path.display())
            }
            Self::LoadConfigFile(source) => write!(f, "加载 JSON 配置失败: {source}"),
            Self::AutoStartDisabled => write!(
                f,
                "server.auto_start_server 为 false，服务端 shell 不会自动启动服务"
            ),
            Self::UnsupportedRuntimeMode(mode) => {
                write!(f, "服务端 shell 不支持远端客户端模式: {mode:?}")
            }
            Self::PrepareStorage { path, .. } => {
                write!(f, "创建存储目录失败: {}", path.display())
            }
            Self::LocalService(source) => write!(f, "本地 Axum 服务失败: {source}"),
        }
    }
}

impl Error for ServerShellError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResolveExecutablePath { source } => Some(source),
            Self::LoadConfigFile(source) => Some(source),
            Self::PrepareStorage { source, .. } => Some(source),
            Self::LocalService(source) => Some(source),
            Self::MissingExecutableDirectory { .. }
            | Self::AutoStartDisabled
            | Self::UnsupportedRuntimeMode(_) => None,
        }
    }
}
