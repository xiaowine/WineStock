use std::{error::Error, fmt};

use winestock_shared::AppConfig;

use crate::{
    auth::{bootstrap_auth, migrate_auth_schema, AuthBootstrap, AuthBootstrapError},
    persistence::{open_sqlite_storage, StorageBootstrapError, StorageRuntime},
};

/// core 根据启动配置完成的初始化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBootstrap {
    pub local_service: Option<LocalServiceBootstrap>,
}

impl CoreBootstrap {
    /// 返回本次配置是否实际初始化了本地服务依赖。
    pub fn initialized_local_service(&self) -> bool {
        self.local_service.is_some()
    }
}

/// 本地 Axum 服务启动前必须准备好的共享状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalServiceBootstrap {
    pub storage: StorageRuntime,
    pub auth: AuthBootstrap,
}

/// core 启动配置初始化错误。
#[derive(Debug)]
pub enum CoreBootstrapError {
    Storage(StorageBootstrapError),
    Auth(AuthBootstrapError),
}

impl fmt::Display for CoreBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(source) => write!(f, "{source}"),
            Self::Auth(source) => write!(f, "{source}"),
        }
    }
}

impl Error for CoreBootstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(source) => Some(source),
            Self::Auth(source) => Some(source),
        }
    }
}

/// 使用已解析配置初始化 core，本函数不查找或读取配置文件。
pub fn bootstrap_from_config(config: &AppConfig) -> Result<CoreBootstrap, CoreBootstrapError> {
    if !config.server.uses_local_service() {
        return Ok(CoreBootstrap {
            local_service: None,
        });
    }

    let storage = open_sqlite_storage(&config.storage).map_err(CoreBootstrapError::Storage)?;

    if config.storage.auto_migrate {
        migrate_auth_schema(&storage.connection)
            .map_err(AuthBootstrapError::Database)
            .map_err(CoreBootstrapError::Auth)?;
    }

    let auth = bootstrap_auth(&storage.connection).map_err(CoreBootstrapError::Auth)?;

    Ok(CoreBootstrap {
        local_service: Some(LocalServiceBootstrap {
            storage: storage.runtime,
            auth,
        }),
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;
    use winestock_shared::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};

    use super::*;

    #[test]
    fn self_hosted_bootstrap_initializes_auth_defaults_and_key() {
        let temp = tempdir().expect("temp dir should exist");
        let config = test_config(
            RuntimeMode::SelfHosted,
            temp.path().join("winestock.sqlite").to_string_lossy(),
            temp.path().join("files").to_string_lossy(),
        );

        let first = bootstrap_from_config(&config)
            .expect("bootstrap should succeed")
            .local_service
            .expect("local service should be initialized");

        assert_eq!(first.auth.settings.access_token_ttl_seconds, 900);
        assert_eq!(first.auth.settings.refresh_token_ttl_seconds, 604800);
        assert!(first.auth.settings.refresh_token_rotation);
        assert_eq!(first.auth.active_signing_key.algorithm, "HS256");
        assert_eq!(
            first.auth.active_signing_key.status,
            crate::SigningKeyStatus::Active
        );
        assert!(first.auth.active_signing_key.key_id.starts_with("ak_"));
        assert!(!first.auth.active_signing_key.key_material.is_empty());
        assert!(first.auth.admin_setup_required);

        let second = bootstrap_from_config(&config)
            .expect("second bootstrap should succeed")
            .local_service
            .expect("local service should be initialized");

        assert_eq!(
            first.auth.active_signing_key.key_id,
            second.auth.active_signing_key.key_id
        );
        assert_eq!(
            first.auth.active_signing_key.key_material,
            second.auth.active_signing_key.key_material
        );

        let conn = Connection::open(&config.storage.database_path).expect("database should open");
        let active_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM auth_signing_keys WHERE status = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("active key count should query");
        assert_eq!(active_count, 1);
    }

    #[test]
    fn client_only_bootstrap_does_not_touch_storage() {
        let temp = tempdir().expect("temp dir should exist");
        let missing_database = temp.path().join("missing").join("winestock.sqlite");
        let config = test_config(
            RuntimeMode::ClientOnly,
            missing_database.to_string_lossy(),
            temp.path().join("files").to_string_lossy(),
        );

        let bootstrap = bootstrap_from_config(&config).expect("client-only should skip storage");

        assert!(!bootstrap.initialized_local_service());
        assert!(!missing_database.exists());
    }

    #[test]
    fn server_mode_bootstrap_uses_local_storage() {
        let temp = tempdir().expect("temp dir should exist");
        let database = temp.path().join("server.sqlite");
        let config = test_config(
            RuntimeMode::ServerMode,
            database.to_string_lossy(),
            temp.path().join("files").to_string_lossy(),
        );

        let bootstrap = bootstrap_from_config(&config).expect("server-mode should use storage");

        assert!(bootstrap.initialized_local_service());
        assert!(database.exists());
    }

    #[test]
    fn auth_defaults_do_not_overwrite_database_managed_settings() {
        let temp = tempdir().expect("temp dir should exist");
        let config = test_config(
            RuntimeMode::SelfHosted,
            temp.path().join("winestock.sqlite").to_string_lossy(),
            temp.path().join("files").to_string_lossy(),
        );

        bootstrap_from_config(&config).expect("first bootstrap should initialize settings");

        let conn = Connection::open(&config.storage.database_path).expect("database should open");
        conn.execute(
            "UPDATE auth_settings SET value = '1200' WHERE key = 'access_token_ttl_seconds'",
            [],
        )
        .expect("setting should update");
        drop(conn);

        let bootstrap = bootstrap_from_config(&config)
            .expect("second bootstrap should preserve settings")
            .local_service
            .expect("local service should be initialized");

        assert_eq!(bootstrap.auth.settings.access_token_ttl_seconds, 1200);
    }

    #[test]
    fn self_hosted_bootstrap_requires_database_directory() {
        let temp = tempdir().expect("temp dir should exist");
        let missing_database = temp.path().join("missing").join("winestock.sqlite");
        let config = test_config(
            RuntimeMode::SelfHosted,
            missing_database.to_string_lossy(),
            temp.path().join("files").to_string_lossy(),
        );

        let error = bootstrap_from_config(&config).expect_err("missing directory should fail");

        assert!(matches!(
            error,
            CoreBootstrapError::Storage(StorageBootstrapError::MissingDatabaseDirectory(_))
        ));
    }

    fn test_config(
        mode: RuntimeMode,
        database_path: impl AsRef<str>,
        files_dir: impl AsRef<str>,
    ) -> AppConfig {
        AppConfig {
            server: ServerConfig {
                mode,
                ..ServerConfig::default()
            },
            storage: StorageConfig {
                database_path: database_path.as_ref().to_owned(),
                files_dir: files_dir.as_ref().to_owned(),
                auto_migrate: true,
            },
        }
    }
}
