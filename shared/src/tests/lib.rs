//! shared 配置和 DTO 测试。

use super::*;

#[test]
fn parses_json_config_shape_from_note() {
    let json = r#"
    {
      "server": {
        "mode": "self-hosted",
        "bind_host": "127.0.0.1",
        "port": 17890,
        "auto_start_server": true,
        "remote_base_url": ""
      },
      "storage": {
        "database_path": "data/winestock.sqlite",
        "files_dir": "data/files",
        "auto_migrate": true
      }
    }
    "#;

    let config = AppConfig::from_json_str(json).expect("config should parse");

    assert_eq!(config.server.mode, RuntimeMode::SelfHosted);
    assert_eq!(config.server.bind_host, "127.0.0.1");
    assert_eq!(config.storage.database_path, "data/winestock.sqlite");
}

#[test]
fn serializes_only_server_and_storage() {
    let json = AppConfig::default()
        .to_json_string_pretty()
        .expect("config should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json should parse");
    let object = value.as_object().expect("root should be object");

    assert_eq!(object.len(), 2);
    assert!(object.contains_key("server"));
    assert!(object.contains_key("storage"));
    assert!(!json.contains("enabled"));
    assert!(!json.contains("jwt"));
    assert!(!json.contains("secret"));
}

#[test]
fn rejects_enabled_flag_in_json_config() {
    let json = r#"
    {
      "server": {
        "mode": "self-hosted",
        "enabled": true,
        "bind_host": "127.0.0.1",
        "port": 17890,
        "auto_start_server": true,
        "remote_base_url": ""
      },
      "storage": {
        "database_path": "data/winestock.sqlite",
        "files_dir": "data/files",
        "auto_migrate": true
      }
    }
    "#;

    let error = AppConfig::from_json_str(json).expect_err("enabled flag must be rejected");
    assert!(error.to_string().contains("enabled"));
}

#[test]
fn rejects_auth_settings_in_json_config() {
    let json = r#"
    {
      "server": {
        "mode": "self-hosted",
        "bind_host": "127.0.0.1",
        "port": 17890,
        "auto_start_server": true,
        "remote_base_url": "",
        "jwt_secret": "do-not-allow"
      },
      "storage": {
        "database_path": "data/winestock.sqlite",
        "files_dir": "data/files",
        "auto_migrate": true
      }
    }
    "#;

    let error = AppConfig::from_json_str(json).expect_err("auth config must be rejected");
    assert!(error.to_string().contains("jwt_secret"));
}

#[test]
fn runtime_mode_reports_storage_need() {
    assert!(!RuntimeMode::ClientOnly.uses_local_service());
    assert!(RuntimeMode::SelfHosted.uses_local_service());
    assert!(RuntimeMode::ServerMode.uses_local_service());
    assert!(!RuntimeMode::ConnectToRemote.uses_local_service());
}
