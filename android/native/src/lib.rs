#![deny(unsafe_op_in_unsafe_fn)]

//! WineStock Android Rust/JNI 适配层。
//!
//! 本 crate 只拥有 JNI JSON、Android 配置映射和应用进程级 core 生命周期；
//! 业务能力仍由 WebView 通过 HTTP 使用 `winestock-core`，不在 JNI 中暴露业务函数。

mod config;
mod contract;
mod engine;
mod error;

#[cfg(target_os = "android")]
mod ffi;

use std::sync::{Mutex, OnceLock};

use config::{require_runtime_config, validate_runtime_request};
use contract::{
    encode_response, EditableRuntimeConfig, NativeInitializeResult, NativeServiceState,
    RuntimeConfigRequest,
};
use engine::NativeEngine;
use error::NativeError;

static ENGINE: OnceLock<Mutex<Option<NativeEngine>>> = OnceLock::new();

/// 初始化进程级 Rust engine；重复调用保持幂等。
pub fn initialize_json() -> String {
    encode_response(with_engine(|_| {
        Ok(NativeInitializeResult { initialized: true })
    }))
}

/// 返回 shared 默认配置的前端可编辑投影。
pub fn default_runtime_config_json() -> String {
    let default = winestock_shared::AppConfig::default();
    encode_response::<EditableRuntimeConfig>(Ok(EditableRuntimeConfig {
        mode: "self-hosted".to_owned(),
        bind_host: default.server.bind_host,
        port: i64::from(default.server.port),
        remote_base_url: default.server.remote_base_url,
    }))
}

/// 使用 shared 与 Android 平台策略校验候选配置。
pub fn validate_runtime_config_json(input: &str) -> String {
    encode_response(parse_request(input).and_then(|request| validate_runtime_request(&request)))
}

/// 启动候选配置对应的本地 core。
pub fn start_local_service_json(input: &str) -> String {
    encode_response(parse_request(input).and_then(|request| {
        let prepared = require_runtime_config(&request)?;
        with_engine(|engine| engine.start(&prepared.app_config))
    }))
}

/// 停止当前本地 core。
pub fn stop_local_service_json() -> String {
    encode_response(with_engine(NativeEngine::stop))
}

/// 使用候选配置重启本地 core。
pub fn restart_local_service_json(input: &str) -> String {
    encode_response(parse_request(input).and_then(|request| {
        let prepared = require_runtime_config(&request)?;
        with_engine(|engine| engine.restart(&prepared.app_config))
    }))
}

/// 查询 native engine 当前观察到的本地服务状态。
pub fn runtime_state_json() -> String {
    encode_response(with_engine(|engine| Ok(engine.state())))
}

/// 显式停止服务并移除 engine，主要供测试和受控进程关闭路径使用。
pub fn shutdown_engine_json() -> String {
    let result = (|| {
        let mutex = ENGINE.get_or_init(|| Mutex::new(None));
        let mut slot = mutex
            .lock()
            .map_err(|_| NativeError::engine_unavailable())?;
        if let Some(mut engine) = slot.take() {
            engine.shutdown()?;
        }
        Ok(NativeServiceState::stopped())
    })();
    encode_response(result)
}

fn parse_request(input: &str) -> Result<RuntimeConfigRequest, NativeError> {
    serde_json::from_str(input).map_err(|_| NativeError::invalid_payload())
}

fn with_engine<T>(
    operation: impl FnOnce(&mut NativeEngine) -> Result<T, NativeError>,
) -> Result<T, NativeError> {
    let mutex = ENGINE.get_or_init(|| Mutex::new(None));
    let mut slot = mutex
        .lock()
        .map_err(|_| NativeError::engine_unavailable())?;
    if slot.is_none() {
        *slot = Some(NativeEngine::new()?);
    }
    operation(slot.as_mut().expect("engine 已完成初始化"))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
