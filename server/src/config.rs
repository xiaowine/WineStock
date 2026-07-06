//! server shell 的配置文件定位、读取和运行前目录准备。
//!
//! 本模块属于 `server shell` 层，固定使用当前可执行文件同目录下的 `data/config.json`。
//! 它负责把配置中的相对存储路径解析到该 `data` 目录，但不决定 core 的业务配置含义。

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use winestock_shared::{AppConfig, StorageConfig};

use crate::error::ServerShellError;

// 服务端 shell 固定把配置放在可执行文件同级 data 目录，避免依赖启动工作目录。
const CONFIG_DATA_DIR: &str = "data";
const CONFIG_FILE_NAME: &str = "config.json";

/// 服务端 shell 完成配置读取后的结果。
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

/// 从可执行文件路径推导固定配置文件路径，不读取或创建文件。
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

/// 缺失配置文件时写入共享默认配置；已有配置不会进入这个函数。
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

    let mut config = AppConfig::default();
    // 配置文件已经位于 data/ 目录内，写入文件的默认存储路径要相对这个目录。
    config.storage.database_path = "winestock.sqlite".to_owned();
    config.storage.files_dir = "files".to_owned();
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

/// 把 JSON 中的相对存储路径解析到配置文件所在目录，传给 core 时必须是可用路径。
fn resolve_storage_paths(config: &mut AppConfig, config_path: &Path) {
    // 配置文件固定在 data/config.json，因此默认相对路径会落在同一个 data 目录中。
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

/// 解析单个配置路径值；绝对路径保持不变，相对路径以配置目录为基准。
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

/// 创建平台壳负责的运行时目录，并把失败路径保留到错误链中。
fn create_dir_all(path: &Path) -> Result<(), ServerShellError> {
    fs::create_dir_all(path).map_err(|source| ServerShellError::PrepareStorage {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
