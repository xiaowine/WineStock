//! Android Kotlin 与 Rust native adapter 之间的版本化 JSON 契约。
//!
//! 本模块只描述配置、校验和本地服务状态，不复制 core HTTP 业务 DTO。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::NativeError;

/// 当前 Kotlin/JNI native JSON 协议版本。
pub const NATIVE_PROTOCOL_VERSION: u32 = 1;

/// 前端可编辑运行配置在 native 边界中的稳定镜像。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditableRuntimeConfig {
    /// shared RuntimeMode 的 kebab-case 字符串。
    pub mode: String,

    /// 本地 Axum 监听地址。
    pub bind_host: String,

    /// 监听端口；先用有符号整数接收，以便稳定返回字段错误。
    pub port: i64,

    /// 远端客户端模式使用的 API 根地址。
    pub remote_base_url: String,
}

/// Android 平台解析并预创建的 app-private 存储路径。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeStoragePaths {
    /// SQLite 主数据库绝对路径。
    pub database_path: String,

    /// 大对象文件仓绝对目录。
    pub files_dir: String,
}

/// 校验、启动或重启本地服务的公共请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeConfigRequest {
    /// 调用方期望的 native 协议版本。
    pub native_protocol_version: u32,

    /// 前端可编辑配置。
    pub config: EditableRuntimeConfig,

    /// Android 平台固定存储路径。
    pub storage: NativeStoragePaths,
}

/// shared/native 权威配置校验结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeValidationResult {
    /// 所有字段和 Android 平台策略是否通过。
    pub valid: bool,

    /// Shell Bridge v1 字段名到用户可见错误说明的映射。
    pub field_errors: BTreeMap<String, Vec<String>>,

    /// 校验通过后由 native 规范化的配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_config: Option<EditableRuntimeConfig>,
}

/// native 本地服务运行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeServiceState {
    /// `stopped`、`running` 或 `failed`。
    pub phase: String,

    /// 操作系统返回的真实监听地址。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_address: Option<String>,

    /// Android WebView 可直接使用的 loopback API 根地址。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,

    /// 当前数据库是否仍需创建首个管理员。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_setup_required: Option<bool>,

    /// 最近一次意外退出错误。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<NativeError>,
}

impl NativeServiceState {
    /// 构造尚未运行本地服务的状态。
    pub fn stopped() -> Self {
        Self {
            phase: "stopped".to_owned(),
            bound_address: None,
            api_base_url: None,
            admin_setup_required: None,
            error: None,
        }
    }

    /// 构造已经记录异常退出的失败状态。
    pub fn failed(error: NativeError) -> Self {
        Self {
            phase: "failed".to_owned(),
            bound_address: None,
            api_base_url: None,
            admin_setup_required: None,
            error: Some(error),
        }
    }
}

/// native engine 初始化结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeInitializeResult {
    /// engine 已可接受配置与生命周期调用。
    pub initialized: bool,
}

/// 所有 JNI 方法共用的版本化响应信封。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeResponse<T: Serialize> {
    native_protocol_version: u32,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<NativeError>,
}

/// 把 native 调用结果序列化为稳定 JSON；序列化自身失败时返回最小安全信封。
pub fn encode_response<T: Serialize>(result: Result<T, NativeError>) -> String {
    let response = match result {
        Ok(value) => NativeResponse {
            native_protocol_version: NATIVE_PROTOCOL_VERSION,
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(error) => NativeResponse {
            native_protocol_version: NATIVE_PROTOCOL_VERSION,
            ok: false,
            result: None,
            error: Some(error),
        },
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {
        format!(
            "{{\"nativeProtocolVersion\":{NATIVE_PROTOCOL_VERSION},\"ok\":false,\"error\":{{\"code\":\"service_start_failed\",\"message\":\"native 响应序列化失败\"}}}}"
        )
    })
}
