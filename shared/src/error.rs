//! 共享配置错误类型。
//!
//! 本模块属于 `shared` 层，描述平台无关配置解析和配置文件读写失败原因。
//! 它保留调用方指定的文件路径，但不决定配置位置或平台启动策略。

use std::{error::Error, fmt, io, path::PathBuf};

/// 启动配置解析错误，区分 JSON 结构错误和字段约束错误。
#[derive(Debug)]
pub enum ConfigParseError {
    /// JSON 语法、字段类型或未知字段错误。
    Json(serde_json::Error),

    /// JSON 已能反序列化，但字段值不满足共享约束。
    Validation(garde::Report),
}

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(source) => write!(f, "{source}"),
            Self::Validation(report) => write!(f, "{report}"),
        }
    }
}

impl Error for ConfigParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(source) => Some(source),
            Self::Validation(_) => None,
        }
    }
}

/// JSON 配置文件加载或缺失文件初始化错误。
#[derive(Debug)]
pub enum ConfigFileError {
    /// 读取已有配置文件失败。
    ReadConfig {
        /// 读取失败的配置文件路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 配置文件内容无法解析为有效启动配置。
    ParseConfig {
        /// 解析失败的配置文件路径。
        path: PathBuf,

        /// JSON 结构或字段约束错误。
        source: ConfigParseError,
    },

    /// 创建默认配置文件的父目录失败。
    CreateConfigDirectory {
        /// 创建失败的目录路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 将默认配置序列化为 JSON 失败。
    SerializeDefaultConfig {
        /// 计划写入的配置文件路径。
        path: PathBuf,

        /// JSON 序列化错误。
        source: serde_json::Error,
    },

    /// 创建或写入默认配置文件失败。
    WriteDefaultConfig {
        /// 写入失败的配置文件路径。
        path: PathBuf,

        /// 底层 IO 错误。
        source: io::Error,
    },
}

impl fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
        }
    }
}

impl Error for ConfigFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadConfig { source, .. }
            | Self::CreateConfigDirectory { source, .. }
            | Self::WriteDefaultConfig { source, .. } => Some(source),
            Self::ParseConfig { source, .. } => Some(source),
            Self::SerializeDefaultConfig { source, .. } => Some(source),
        }
    }
}
