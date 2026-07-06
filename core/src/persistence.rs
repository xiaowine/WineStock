use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use winestock_shared::StorageConfig;

/// core 打开本地存储后的运行时路径信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRuntime {
    pub database_path: PathBuf,
    pub files_dir: PathBuf,
}

pub(crate) struct InitializedStorage {
    pub runtime: StorageRuntime,
    pub connection: Connection,
}

/// 本地存储初始化错误。
#[derive(Debug)]
pub enum StorageBootstrapError {
    EmptyDatabasePath,
    EmptyFilesDir,
    MissingDatabaseDirectory(PathBuf),
    OpenDatabase {
        path: PathBuf,
        source: rusqlite::Error,
    },
    ConfigureDatabase {
        path: PathBuf,
        source: rusqlite::Error,
    },
}

impl fmt::Display for StorageBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDatabasePath => write!(f, "storage.database_path must not be empty"),
            Self::EmptyFilesDir => write!(f, "storage.files_dir must not be empty"),
            Self::MissingDatabaseDirectory(path) => {
                write!(f, "database directory does not exist: {}", path.display())
            }
            Self::OpenDatabase { path, .. } => {
                write!(f, "failed to open SQLite database: {}", path.display())
            }
            Self::ConfigureDatabase { path, .. } => {
                write!(f, "failed to configure SQLite database: {}", path.display())
            }
        }
    }
}

impl Error for StorageBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenDatabase { source, .. } | Self::ConfigureDatabase { source, .. } => {
                Some(source)
            }
            _ => None,
        }
    }
}

pub(crate) fn open_sqlite_storage(
    config: &StorageConfig,
) -> Result<InitializedStorage, StorageBootstrapError> {
    let database_path = path_from_config_value(&config.database_path);
    let files_dir = path_from_config_value(&config.files_dir);

    if database_path.as_os_str().is_empty() {
        return Err(StorageBootstrapError::EmptyDatabasePath);
    }

    if files_dir.as_os_str().is_empty() {
        return Err(StorageBootstrapError::EmptyFilesDir);
    }

    if let Some(parent) = meaningful_parent(&database_path) {
        if !parent.exists() {
            return Err(StorageBootstrapError::MissingDatabaseDirectory(
                parent.to_path_buf(),
            ));
        }
    }

    let connection =
        Connection::open(&database_path).map_err(|source| StorageBootstrapError::OpenDatabase {
            path: database_path.clone(),
            source,
        })?;

    // SQLite 的运行时 PRAGMA 集中在存储初始化层，避免散落到业务处理函数。
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA journal_mode = WAL;
            PRAGMA wal_autocheckpoint = 1000;
            "#,
        )
        .map_err(|source| StorageBootstrapError::ConfigureDatabase {
            path: database_path.clone(),
            source,
        })?;

    Ok(InitializedStorage {
        runtime: StorageRuntime {
            database_path,
            files_dir,
        },
        connection,
    })
}

fn path_from_config_value(value: &str) -> PathBuf {
    PathBuf::from(value.trim())
}

fn meaningful_parent(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
}
