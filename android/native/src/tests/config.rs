//! Android native 配置映射测试。

use std::collections::BTreeMap;

use tempfile::tempdir;

use crate::{
    config::{require_runtime_config, validate_runtime_request},
    contract::{
        EditableRuntimeConfig, NativeStoragePaths, RuntimeConfigRequest, NATIVE_PROTOCOL_VERSION,
    },
};

fn request(mode: &str) -> RuntimeConfigRequest {
    let temp = tempdir().expect("temp dir should exist").keep();
    RuntimeConfigRequest {
        native_protocol_version: NATIVE_PROTOCOL_VERSION,
        config: EditableRuntimeConfig {
            mode: mode.to_owned(),
            bind_host: "127.0.0.1".to_owned(),
            port: 17890,
            remote_base_url: String::new(),
        },
        storage: NativeStoragePaths {
            database_path: temp.join("winestock.sqlite").to_string_lossy().into_owned(),
            files_dir: temp.join("files").to_string_lossy().into_owned(),
        },
    }
}

#[test]
fn accepts_android_loopback_self_hosted_config() {
    let request = request("self-hosted");

    let prepared = require_runtime_config(&request).expect("config should be valid");

    assert_eq!(prepared.app_config.server.bind_host, "127.0.0.1");
    assert_eq!(prepared.app_config.server.port, 17890);
    assert!(prepared.app_config.server.auto_start_server);
}

#[test]
fn rejects_non_loopback_and_server_mode() {
    let mut non_loopback = request("self-hosted");
    non_loopback.config.bind_host = "0.0.0.0".to_owned();
    let validation = validate_runtime_request(&non_loopback).expect("validation should run");
    assert_eq!(
        validation.field_errors.keys().cloned().collect::<Vec<_>>(),
        vec!["bindHost".to_owned()]
    );

    let server_mode = request("server-mode");
    let validation = validate_runtime_request(&server_mode).expect("validation should run");
    assert_eq!(
        validation.field_errors.keys().cloned().collect::<Vec<_>>(),
        vec!["mode".to_owned()]
    );
}

#[test]
fn normalizes_remote_url_and_rejects_credentials() {
    let mut remote = request("connect-to-remote");
    remote.config.remote_base_url = "https://example.test/api/".to_owned();

    let validation = validate_runtime_request(&remote).expect("validation should run");
    assert!(validation.valid);
    assert_eq!(
        validation
            .normalized_config
            .expect("normalized config should exist")
            .remote_base_url,
        "https://example.test/api"
    );

    remote.config.remote_base_url = "https://user:pass@example.test".to_owned();
    let validation = validate_runtime_request(&remote).expect("validation should run");
    assert_eq!(
        validation.field_errors,
        BTreeMap::from([(
            "remoteBaseUrl".to_owned(),
            vec!["远端服务地址不能包含凭据、查询参数或 hash".to_owned()],
        )])
    );
}
