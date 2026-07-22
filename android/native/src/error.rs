//! native adapter 的稳定错误与 core 错误映射。
//!
//! 完整 Rust source chain 只用于平台日志；跨 JNI 只返回稳定错误码和安全中文说明。

use std::io;

use serde::Serialize;
use winestock_core::{
    CoreBootstrapError, LocalServiceRuntimeError, ServerStartError, StorageBootstrapError,
};

/// Kotlin 和 Shell Bridge 可安全消费的 native 错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeError {
    /// Shell Bridge 使用的稳定错误码。
    pub code: String,

    /// 不含本机路径、SQL 或 backtrace 的中文说明。
    pub message: String,

    /// 对应的 EditableRuntimeConfig 字段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl NativeError {
    /// 构造无字段归属的错误。
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
            field: None,
        }
    }

    /// 构造可定位到运行配置字段的错误。
    pub fn field(code: &str, message: &str, field: &str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
            field: Some(field.to_owned()),
        }
    }

    /// native 协议版本不匹配。
    pub fn version_mismatch() -> Self {
        Self::new("bridge_version_mismatch", "Android native 协议版本不兼容")
    }

    /// JNI JSON 无法解析或缺少字段。
    pub fn invalid_payload() -> Self {
        Self::new("invalid_bridge_payload", "Android native 请求结构无效")
    }

    /// 配置校验未通过。
    pub fn config_invalid(field: Option<&str>) -> Self {
        match field {
            Some(field) => Self::field("config_invalid", "运行配置无效", field),
            None => Self::new("config_invalid", "运行配置无效"),
        }
    }

    /// native engine 锁或 Runtime 初始化失败。
    pub fn engine_unavailable() -> Self {
        Self::new(
            "native_library_unavailable",
            "Android 本地服务组件无法初始化",
        )
    }
}

impl From<LocalServiceRuntimeError> for NativeError {
    fn from(error: LocalServiceRuntimeError) -> Self {
        match error {
            LocalServiceRuntimeError::Bootstrap(source) => map_bootstrap_error(source),
            LocalServiceRuntimeError::LocalServiceNotInitialized => {
                Self::new("service_start_failed", "core 未初始化本地服务运行状态")
            }
            LocalServiceRuntimeError::Server(source) => map_server_error(source),
            LocalServiceRuntimeError::Task(_) => {
                Self::new("service_crashed", "本地服务任务异常结束")
            }
        }
    }
}

fn map_bootstrap_error(error: CoreBootstrapError) -> NativeError {
    match error {
        CoreBootstrapError::Storage(StorageBootstrapError::EmptyDatabasePath)
        | CoreBootstrapError::Storage(StorageBootstrapError::EmptyFilesDir)
        | CoreBootstrapError::Storage(StorageBootstrapError::MissingDatabaseDirectory(_)) => {
            NativeError::new("storage_unavailable", "本地存储目录不可用")
        }
        CoreBootstrapError::Storage(StorageBootstrapError::OpenDatabase { .. })
        | CoreBootstrapError::Storage(StorageBootstrapError::ConfigureDatabase { .. }) => {
            NativeError::new("database_open_failed", "无法打开或配置本地数据库")
        }
        CoreBootstrapError::Storage(StorageBootstrapError::MigrateDatabase { .. }) => {
            NativeError::new("migration_failed", "本地数据库升级失败")
        }
        _ => NativeError::new("service_start_failed", "本地服务初始化失败"),
    }
}

fn map_server_error(error: ServerStartError) -> NativeError {
    match error {
        ServerStartError::LocalServiceUnavailable => NativeError::field(
            "unsupported_runtime_mode",
            "当前运行模式不启动本地服务",
            "mode",
        ),
        ServerStartError::InvalidBindHost { .. } => {
            NativeError::field("invalid_bind_host", "本地服务监听地址无效", "bindHost")
        }
        ServerStartError::Bind { source, .. } if source.kind() == io::ErrorKind::AddrInUse => {
            NativeError::field("port_in_use", "本地服务端口已被占用", "port")
        }
        ServerStartError::Serve(_) => {
            NativeError::new("service_crashed", "本地服务运行过程中异常停止")
        }
        ServerStartError::Bind { .. } | ServerStartError::LocalAddr(_) => {
            NativeError::new("service_start_failed", "本地服务网络启动失败")
        }
    }
}
