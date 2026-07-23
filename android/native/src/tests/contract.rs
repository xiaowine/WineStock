//! Android native JSON 响应契约测试。

use serde_json::Value;

use crate::{
    contract::NATIVE_PROTOCOL_VERSION, default_runtime_config_json, initialize_json,
    shutdown_engine_json, start_local_service_json,
};

#[test]
fn response_envelope_contains_protocol_and_result() {
    let value: Value = serde_json::from_str(&initialize_json()).expect("response should parse");

    assert_eq!(
        value["nativeProtocolVersion"].as_u64(),
        Some(u64::from(NATIVE_PROTOCOL_VERSION))
    );
    assert_eq!(value["ok"].as_bool(), Some(true));
    assert_eq!(value["result"]["initialized"].as_bool(), Some(true));
}

#[test]
fn default_config_comes_from_shared_defaults() {
    let value: Value =
        serde_json::from_str(&default_runtime_config_json()).expect("response should parse");

    assert_eq!(value["result"]["mode"], "self-hosted");
    assert_eq!(value["result"]["bindHost"], "127.0.0.1");
    assert_eq!(value["result"]["port"], 17890);
}

#[test]
fn zero_port_start_returns_matching_nonzero_runtime_addresses() {
    let temp = tempfile::tempdir().expect("temporary storage should exist");
    let request = serde_json::json!({
        "nativeProtocolVersion": NATIVE_PROTOCOL_VERSION,
        "config": {
            "mode": "self-hosted",
            "bindHost": "127.0.0.1",
            "port": 0,
            "remoteBaseUrl": ""
        },
        "storage": {
            "databasePath": temp.path().join("winestock.sqlite"),
            "filesDir": temp.path().join("files")
        }
    });

    let response = start_local_service_json(&request.to_string());
    let _ = shutdown_engine_json();
    let value: Value = serde_json::from_str(&response).expect("response should parse");

    assert_eq!(value["ok"].as_bool(), Some(true));
    let bound = value["result"]["boundAddress"]
        .as_str()
        .expect("bound address should exist")
        .parse::<std::net::SocketAddr>()
        .expect("bound address should parse");
    let api = value["result"]["apiBaseUrl"]
        .as_str()
        .expect("API base URL should exist");
    assert_ne!(bound.port(), 0);
    assert_eq!(api, format!("http://127.0.0.1:{}", bound.port()));
}
