use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use winestock_shared::{AppConfig, StorageConfig};

use crate::error::ServerShellError;

const CONFIG_DATA_DIR: &str = "data";
const CONFIG_FILE_NAME: &str = "config.json";

pub(crate) struct LoadedConfig {
    /// 已解析并完成相对路径补齐的启动配置。
    pub config: AppConfig,

    /// 本次加载是否因为配置文件缺失而创建了默认配置。
    pub created_default: bool,
}

/// 返回服务端 shell 固定配置文件路径。
pub(crate) fn fixed_config_path() -> Result<PathBuf, ServerShellError> {
    let exe_path =
        env::current_exe().map_err(|source| ServerShellError::ResolveExecutablePath { source })?;

    config_path_from_exe_path(&exe_path)
}

fn config_path_from_exe_path(exe_path: &Path) -> Result<PathBuf, ServerShellError> {
    let exe_dir = exe_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| ServerShellError::MissingExecutableDirectory {
            path: exe_path.to_path_buf(),
        })?;

    Ok(exe_dir.join(CONFIG_DATA_DIR).join(CONFIG_FILE_NAME))
}

/// 读取或创建配置文件，并把存储路径解析到配置文件所在目录。
pub(crate) fn load_config(config_path: &Path) -> Result<LoadedConfig, ServerShellError> {
    let (mut config, created_default) = match fs::read_to_string(config_path) {
        Ok(content) => (
            AppConfig::from_json_str(&content).map_err(|source| ServerShellError::ParseConfig {
                path: config_path.to_path_buf(),
                source,
            })?,
            false,
        ),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            (create_default_config_file(config_path)?, true)
        }
        Err(source) => {
            return Err(ServerShellError::ReadConfig {
                path: config_path.to_path_buf(),
                source,
            });
        }
    };

    resolve_storage_paths(&mut config, config_path);
    Ok(LoadedConfig {
        config,
        created_default,
    })
}

fn create_default_config_file(config_path: &Path) -> Result<AppConfig, ServerShellError> {
    if let Some(parent) = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| ServerShellError::CreateConfigDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let config = AppConfig::default();
    let mut content = config.to_json_string_pretty().map_err(|source| {
        ServerShellError::SerializeDefaultConfig {
            path: config_path.to_path_buf(),
            source,
        }
    })?;
    content.push('\n');

    // 配置文件属于平台壳管理；缺失时写入共享默认配置，避免用户手动创建。
    fs::write(config_path, content).map_err(|source| ServerShellError::WriteDefaultConfig {
        path: config_path.to_path_buf(),
        source,
    })?;

    Ok(config)
}

/// 校验服务端 shell 只能运行需要本地服务的模式。
pub(crate) fn ensure_server_runtime(config: &AppConfig) -> Result<(), ServerShellError> {
    if !config.server.auto_start_server {
        return Err(ServerShellError::AutoStartDisabled);
    }

    if !config.server.mode.uses_local_service() {
        return Err(ServerShellError::UnsupportedRuntimeMode(config.server.mode));
    }

    Ok(())
}

fn resolve_storage_paths(config: &mut AppConfig, config_path: &Path) {
    let base_dir = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    config.storage.database_path = resolve_path(base_dir, &config.storage.database_path)
        .to_string_lossy()
        .into_owned();
    config.storage.files_dir = resolve_path(base_dir, &config.storage.files_dir)
        .to_string_lossy()
        .into_owned();
}

fn resolve_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value.trim());
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

/// 在 core 打开数据库之前准备数据库父目录和文件目录。
pub(crate) fn prepare_storage_dirs(storage: &StorageConfig) -> Result<(), ServerShellError> {
    let database_path = Path::new(&storage.database_path);
    if let Some(parent) = database_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        create_dir_all(parent)?;
    }

    create_dir_all(Path::new(&storage.files_dir))?;
    Ok(())
}

fn create_dir_all(path: &Path) -> Result<(), ServerShellError> {
    fs::create_dir_all(path).map_err(|source| ServerShellError::PrepareStorage {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
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
        let config_path = temp.path().join("missing").join("config.json");

        let loaded = load_config(&config_path).expect("missing config should be created");

        assert!(loaded.created_default);
        assert!(config_path.exists());
        assert_eq!(loaded.config.server, ServerConfig::default());
        assert_eq!(
            PathBuf::from(&loaded.config.storage.database_path),
            temp.path().join("missing").join("winestock.sqlite")
        );
        assert_eq!(
            PathBuf::from(&loaded.config.storage.files_dir),
            temp.path().join("missing").join("files")
        );

        let file_content =
            fs::read_to_string(&config_path).expect("config file should be readable");
        let file_config =
            AppConfig::from_json_str(&file_content).expect("created config should parse");
        assert_eq!(file_config.storage.database_path, "winestock.sqlite");
        assert_eq!(file_config.storage.files_dir, "files");
    }

    #[test]
    fn existing_json_config_is_not_overwritten() {
        let temp = tempdir().expect("temp dir should exist");
        let config_path = temp.path().join("config.json");
        let mut custom_config = AppConfig::default();
        custom_config.server.port = 19001;
        let mut custom_content = custom_config
            .to_json_string_pretty()
            .expect("custom config should serialize");
        custom_content.push('\n');
        fs::write(&config_path, &custom_content).expect("custom config should write");

        let loaded = load_config(&config_path).expect("existing config should load");

        assert!(!loaded.created_default);
        assert_eq!(loaded.config.server.port, 19001);
        assert_eq!(
            fs::read_to_string(&config_path).expect("custom config should remain readable"),
            custom_content
        );
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
}
