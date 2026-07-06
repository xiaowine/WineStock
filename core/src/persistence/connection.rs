//! SQLite 连接、PRAGMA 配置和 migration 执行入口。
//!
//! 本模块属于 `core` 的持久化层，负责把平台壳解析好的 `StorageConfig`
//! 转换成 SeaORM 连接和运行时路径信息。目录创建属于平台壳，不在这里做。

use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use winestock_shared::StorageConfig;

use super::migration::Migrator;

/// core 打开本地存储后的运行时状态。
#[derive(Debug, Clone)]
pub struct StorageRuntime {
    /// SQLite 主数据库文件路径，由平台壳解析后传入 core。
    pub database_path: PathBuf,

    /// 大对象文件目录；SQLite 只保存文件元数据。
    pub files_dir: PathBuf,

    /// SeaORM 数据库连接，供 repository 和后续 Axum 状态复用。
    pub database: DatabaseConnection,
}

/// 本地存储初始化错误。
#[derive(Debug)]
pub enum StorageBootstrapError {
    /// 配置中的数据库路径为空。
    EmptyDatabasePath,

    /// 配置中的文件目录为空。
    EmptyFilesDir,

    /// core 不创建数据库父目录，目录准备属于平台壳职责。
    MissingDatabaseDirectory(PathBuf),

    /// SQLite 文件或连接池打开失败。
    OpenDatabase {
        /// 打开失败的数据库路径。
        path: PathBuf,

        /// SQLx 返回的底层错误。
        source: sqlx::Error,
    },

    /// SQLite PRAGMA 初始化失败。
    ConfigureDatabase {
        /// 配置失败的数据库路径。
        path: PathBuf,

        /// SeaORM 返回的底层错误。
        source: DbErr,
    },

    /// SeaORM 迁移执行失败。
    MigrateDatabase {
        /// 迁移失败的数据库路径。
        path: PathBuf,

        /// SeaORM 迁移返回的底层错误。
        source: DbErr,
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
            Self::MigrateDatabase { path, .. } => {
                write!(f, "failed to migrate SQLite database: {}", path.display())
            }
        }
    }
}

impl Error for StorageBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenDatabase { source, .. } => Some(source),
            Self::ConfigureDatabase { source, .. } | Self::MigrateDatabase { source, .. } => {
                Some(source)
            }
            _ => None,
        }
    }
}

/// 根据平台壳传入的存储配置打开 SQLite，并集中应用基础运行时设置。
pub(crate) async fn open_sqlite_storage(
    config: &StorageConfig,
) -> Result<StorageRuntime, StorageBootstrapError> {
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

    let database = connect_sqlite(&database_path).await?;
    configure_sqlite(&database, &database_path).await?;

    Ok(StorageRuntime {
        database_path,
        files_dir,
        database,
    })
}

/// 执行 core 内置 SeaORM 迁移，调用方通过 `StorageConfig.auto_migrate` 决定是否调用。
pub(crate) async fn migrate_storage_schema(
    storage: &StorageRuntime,
) -> Result<(), StorageBootstrapError> {
    Migrator::up(&storage.database, None)
        .await
        .map_err(|source| StorageBootstrapError::MigrateDatabase {
            path: storage.database_path.clone(),
            source,
        })
}

/// 打开 SQLite 连接池，并为连接池中的每条连接设置一致的 PRAGMA。
async fn connect_sqlite(database_path: &Path) -> Result<DatabaseConnection, StorageBootstrapError> {
    // PRAGMA 放在连接选项里，确保连接池中新建的每条 SQLite 连接都继承同一运行策略。
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(5_000))
        .pragma("wal_autocheckpoint", "1000");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|source| StorageBootstrapError::OpenDatabase {
            path: database_path.to_path_buf(),
            source,
        })?;

    Ok(sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

async fn configure_sqlite(
    database: &DatabaseConnection,
    database_path: &Path,
) -> Result<(), StorageBootstrapError> {
    // SQLx 连接选项负责每条连接的 PRAGMA；这里再执行一次，让 WAL 等持久设置立即落盘。
    for statement in [
        "PRAGMA foreign_keys = ON",
        "PRAGMA busy_timeout = 5000",
        "PRAGMA journal_mode = WAL",
        "PRAGMA wal_autocheckpoint = 1000",
    ] {
        database
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                statement.to_owned(),
            ))
            .await
            .map_err(|source| StorageBootstrapError::ConfigureDatabase {
                path: database_path.to_path_buf(),
                source,
            })?;
    }

    Ok(())
}

/// 将共享配置中的路径字符串裁剪空白后转成 PathBuf，不在 core 内补相对路径。
fn path_from_config_value(value: &str) -> PathBuf {
    PathBuf::from(value.trim())
}

/// 返回需要由平台壳预先创建的父目录；裸文件名不做目录存在性检查。
fn meaningful_parent(path: &Path) -> Option<&Path> {
    // 相对裸文件名没有可校验父目录，交给 SQLite 在当前工作目录创建文件。
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use tempfile::tempdir;
    use winestock_shared::StorageConfig;

    use super::*;

    #[tokio::test]
    async fn opens_sqlite_database_with_wal_pragmas() {
        let temp = tempdir().expect("temp dir should exist");
        let config = StorageConfig {
            database_path: temp
                .path()
                .join("winestock.sqlite")
                .to_string_lossy()
                .into_owned(),
            files_dir: temp.path().join("files").to_string_lossy().into_owned(),
            auto_migrate: true,
        };

        let storage = open_sqlite_storage(&config)
            .await
            .expect("storage should open");

        assert!(storage.database_path.exists());
        assert_eq!(
            query_string(&storage.database, "PRAGMA journal_mode", "journal_mode").await,
            "wal"
        );
        assert_eq!(
            query_i64(&storage.database, "PRAGMA foreign_keys", "foreign_keys").await,
            1
        );
    }

    #[tokio::test]
    async fn migration_is_idempotent_and_creates_v1_tables() {
        let temp = tempdir().expect("temp dir should exist");
        let config = StorageConfig {
            database_path: temp
                .path()
                .join("winestock.sqlite")
                .to_string_lossy()
                .into_owned(),
            files_dir: temp.path().join("files").to_string_lossy().into_owned(),
            auto_migrate: true,
        };
        let storage = open_sqlite_storage(&config)
            .await
            .expect("storage should open");

        migrate_storage_schema(&storage)
            .await
            .expect("first migration should succeed");
        migrate_storage_schema(&storage)
            .await
            .expect("second migration should be idempotent");

        for table in [
            "auth_users",
            "auth_roles",
            "auth_user_role_assignments",
            "auth_permissions",
            "auth_role_permission_assignments",
            "auth_settings",
            "auth_signing_keys",
            "auth_refresh_tokens",
            "storage_file_objects",
        ] {
            let sql = format!(
                "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = '{table}'"
            );
            assert_eq!(query_i64(&storage.database, &sql, "count").await, 1);
        }
    }

    async fn query_string(database: &DatabaseConnection, sql: &str, column: &str) -> String {
        database
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_owned(),
            ))
            .await
            .expect("query should execute")
            .expect("row should exist")
            .try_get("", column)
            .expect("column should decode")
    }

    async fn query_i64(database: &DatabaseConnection, sql: &str, column: &str) -> i64 {
        database
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_owned(),
            ))
            .await
            .expect("query should execute")
            .expect("row should exist")
            .try_get("", column)
            .expect("column should decode")
    }
}
