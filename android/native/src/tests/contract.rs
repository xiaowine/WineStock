//! Android native JSON 响应契约测试。

use serde_json::Value;

use crate::{contract::NATIVE_PROTOCOL_VERSION, default_runtime_config_json, initialize_json};

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
