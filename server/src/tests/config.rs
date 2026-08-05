//! server shell 配置加载测试。

use std::fs;
use tempfile::tempdir;
use winestock_shared::{RuntimeMode, ServerConfig};

use super::*;

#[test]
fn fixed_config_path_uses_data_dir_next_to_executable() {
    let temp = tempdir().expect("temp dir should exist");
    let exe_path = temp.path().join("bin").join("winestock-server.exe");

    let config_path =
        config_path_from_exe_path(&exe_path).expect("exe path should resolve config path");

    assert_eq!(
        config_path,
        temp.path().join("bin").join("data").join("config.json")
    );
}

#[test]
fn resolves_relative_storage_paths_from_config_directory() {
    let temp = tempdir().expect("temp dir should exist");
    let config_path = temp.path().join("bin").join("data").join("config.json");
    let mut config = AppConfig {
        server: ServerConfig::default(),
        storage: StorageConfig {
            database_path: "winestock.sqlite".to_owned(),
            files_dir: "files".to_owned(),
            auto_migrate: true,
        },
    };

    resolve_storage_paths(&mut config, &config_path);

    assert_eq!(
        PathBuf::from(config.storage.database_path),
        temp.path()
            .join("bin")
            .join("data")
            .join("winestock.sqlite")
    );
    assert_eq!(
        PathBuf::from(config.storage.files_dir),
        temp.path().join("bin").join("data").join("files")
    );
}

#[test]
fn creates_default_json_config_when_missing() {
    let temp = tempdir().expect("temp dir should exist");
    let config_path = temp.path().join("bin").join("data").join("config.json");

    let loaded = load_config(&config_path).expect("missing config should be created");

    assert!(loaded.created_default);
    assert!(config_path.exists());
    assert_eq!(loaded.config.server.mode, RuntimeMode::ServerMode);
    assert_eq!(loaded.config.server.bind_host, "127.0.0.1");
    assert_eq!(loaded.config.server.port, ServerConfig::default().port);
    assert!(loaded.config.server.auto_start_server);
    assert_eq!(
        PathBuf::from(&loaded.config.storage.database_path),
        temp.path()
            .join("bin")
            .join("data")
            .join("winestock.sqlite")
    );
    assert_eq!(
        PathBuf::from(&loaded.config.storage.files_dir),
        temp.path().join("bin").join("data").join("files")
    );

    let file_content = fs::read_to_string(&config_path).expect("config file should be readable");
    let file_config = AppConfig::from_json_str(&file_content).expect("created config should parse");
    assert_eq!(file_config.storage.database_path, "winestock.sqlite");
    assert_eq!(file_config.storage.files_dir, "files");
}

#[test]
fn rejects_remote_only_runtime_modes() {
    let mut config = AppConfig::default();
    config.server.mode = RuntimeMode::ClientOnly;

    let error = ensure_server_runtime(&config).expect_err("client-only should fail");

    assert!(matches!(
        error,
        ServerShellError::UnsupportedRuntimeMode(RuntimeMode::ClientOnly)
    ));
}
