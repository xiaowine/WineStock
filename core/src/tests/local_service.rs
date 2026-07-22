//! core 本地服务运行句柄测试。

use std::io;

use tempfile::tempdir;
use tokio::net::TcpListener;
use winestock_shared::{AppConfig, RuntimeMode, ServerConfig, StorageConfig};

use super::*;

fn local_config(database_dir: &std::path::Path) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            mode: RuntimeMode::SelfHosted,
            bind_host: "127.0.0.1".to_owned(),
            port: 0,
            ..ServerConfig::default()
        },
        storage: StorageConfig {
            database_path: database_dir
                .join("winestock.sqlite")
                .to_string_lossy()
                .into_owned(),
            files_dir: database_dir.join("files").to_string_lossy().into_owned(),
            auto_migrate: true,
        },
    }
}

#[tokio::test]
async fn starts_and_gracefully_releases_bound_port() {
    let temp = tempdir().expect("temp dir should exist");
    let running = start_local_service(&local_config(temp.path()))
        .await
        .expect("local service should start");
    let bound_addr = running.info().bound_addr;

    assert!(bound_addr.ip().is_loopback());
    assert_ne!(bound_addr.port(), 0);
    assert!(!running.is_finished());

    running
        .shutdown()
        .await
        .expect("local service should stop gracefully");
    let rebound = TcpListener::bind(bound_addr)
        .await
        .expect("shutdown should release bound port");
    drop(rebound);
}

#[tokio::test]
async fn reports_port_conflict_before_bootstrap() {
    let temp = tempdir().expect("temp dir should exist");
    let occupied = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test port should bind");
    let mut config = local_config(temp.path());
    config.server.port = occupied
        .local_addr()
        .expect("test address should resolve")
        .port();

    let error = start_local_service(&config)
        .await
        .expect_err("occupied port should fail");

    assert!(matches!(
        error,
        LocalServiceRuntimeError::Server(ServerStartError::Bind { source, .. })
            if source.kind() == io::ErrorKind::AddrInUse
    ));
    assert!(!temp.path().join("winestock.sqlite").exists());
}

#[tokio::test]
async fn rejects_remote_only_mode() {
    let temp = tempdir().expect("temp dir should exist");
    let mut config = local_config(temp.path());
    config.server.mode = RuntimeMode::ClientOnly;

    let error = start_local_service(&config)
        .await
        .expect_err("remote mode should not start local service");

    assert!(matches!(
        error,
        LocalServiceRuntimeError::Server(ServerStartError::LocalServiceUnavailable)
    ));
}
