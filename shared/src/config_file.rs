//! 平台无关 JSON 配置文件加载与缺失文件初始化。
//!
//! 本模块属于 `shared` 层，只处理调用方指定路径上的配置文件读写。
//! 它不决定平台 shell 的配置位置、默认路径策略或服务生命周期。

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

use crate::{AppConfig, ConfigFileError};

/// JSON 配置文件加载结果。
#[derive(Debug)]
pub struct LoadedJsonConfig {
    /// 从已有文件解析或在文件缺失时写入的配置。
    pub config: AppConfig,

    /// 本次调用是否创建了默认配置文件。
    pub created_default: bool,
}

/// 读取调用方指定的 JSON 配置，文件缺失时写入调用方提供的默认配置。
///
/// 只有读取失败原因为 `NotFound` 时才会创建文件；其他读取错误会直接返回。
/// 默认文件使用格式化 JSON 和末尾换行，并通过 `create_new` 保证不会覆盖已有文件。
pub fn load_or_create_json_config(
    path: &Path,
    default_config: &AppConfig,
) -> Result<LoadedJsonConfig, ConfigFileError> {
    match fs::read_to_string(path) {
        Ok(content) => {
            let config = AppConfig::from_json_str(&content).map_err(|source| {
                ConfigFileError::ParseConfig {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            Ok(LoadedJsonConfig {
                config,
                created_default: false,
            })
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            create_default_json_config(path, default_config)
        }
        Err(source) => Err(ConfigFileError::ReadConfig {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// 创建缺失的配置文件，并保留调用方传入的默认配置值。
fn create_default_json_config(
    path: &Path,
    default_config: &AppConfig,
) -> Result<LoadedJsonConfig, ConfigFileError> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|source| ConfigFileError::CreateConfigDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut content = default_config.to_json_string_pretty().map_err(|source| {
        ConfigFileError::SerializeDefaultConfig {
            path: path.to_path_buf(),
            source,
        }
    })?;
    content.push('\n');

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| ConfigFileError::WriteDefaultConfig {
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(content.as_bytes())
        .map_err(|source| ConfigFileError::WriteDefaultConfig {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(LoadedJsonConfig {
        config: default_config.clone(),
        created_default: true,
    })
}
