//! 共享配置错误类型。
//!
//! 本模块属于 `shared` 层，描述平台无关配置解析失败原因。
//! 它不决定配置文件位置、默认文件创建或平台启动策略。

use std::{error::Error, fmt};

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
