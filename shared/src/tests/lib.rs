//! shared 配置测试。

use super::*;
use std::fs;
use tempfile::tempdir;

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

#[test]
fn json_config_validation_rejects_invalid_field_values() {
    let json = r#"
    {
      "server": {
        "mode": "self-hosted",
        "bind_host": "localhost",
        "port": 0,
        "auto_start_server": true,
        "remote_base_url": "ftp://example.test"
      },
      "storage": {
        "database_path": "",
        "files_dir": "data/files",
        "auto_migrate": true
      }
    }
    "#;

    let error = AppConfig::from_json_str(json).expect_err("invalid config should be rejected");
    let message = error.to_string();
    assert!(message.contains("bind_host"));
    assert!(message.contains("port"));
    assert!(message.contains("database_path"));
}

#[test]
fn exposes_structured_validation_issue_paths() {
    let mut config = AppConfig::default();
    config.server.bind_host = "localhost".to_owned();
    config.storage.database_path = " ".to_owned();

    let issues = config.validation_issues();

    assert!(issues.iter().any(|issue| issue.path == "server.bind_host"));
    assert!(issues
        .iter()
        .any(|issue| issue.path == "storage.database_path"));
    assert!(issues.iter().all(|issue| !issue.message.is_empty()));
}

#[test]
fn creates_caller_default_json_config_when_missing() {
    let temp = tempdir().expect("temp dir should exist");
    let config_path = temp.path().join("nested").join("config.json");
    let mut default_config = AppConfig::default();
    default_config.server.port = 19001;

    let loaded = load_or_create_json_config(&config_path, &default_config)
        .expect("missing config should be created");

    assert!(loaded.created_default);
    assert_eq!(loaded.config, default_config);
    let content = fs::read_to_string(&config_path).expect("created config should be readable");
    assert!(content.ends_with('\n'));
    assert_eq!(
        AppConfig::from_json_str(&content).expect("created config should parse"),
        default_config
    );
}

#[test]
fn preserves_existing_json_config() {
    let temp = tempdir().expect("temp dir should exist");
    let config_path = temp.path().join("config.json");
    let mut existing_config = AppConfig::default();
    existing_config.server.port = 19002;
    let mut existing_content = existing_config
        .to_json_string_pretty()
        .expect("existing config should serialize");
    existing_content.push('\n');
    fs::write(&config_path, &existing_content).expect("existing config should write");

    let loaded = load_or_create_json_config(&config_path, &AppConfig::default())
        .expect("existing config should load");

    assert!(!loaded.created_default);
    assert_eq!(loaded.config, existing_config);
    assert_eq!(
        fs::read_to_string(&config_path).expect("existing config should remain readable"),
        existing_content
    );
}
