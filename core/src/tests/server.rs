//! core 服务绑定测试。

use std::io;

use winestock_shared::{RuntimeMode, ServerConfig};

use super::*;

#[tokio::test]
async fn bind_server_uses_configured_loopback_and_allocated_port() {
    let config = ServerConfig {
        mode: RuntimeMode::SelfHosted,
        bind_host: "127.0.0.1".to_owned(),
        port: 0,
        ..ServerConfig::default()
    };

    let bound = bind_server(&config).await.expect("server should bind");

    assert!(bound.bound_addr().ip().is_loopback());
    assert_ne!(bound.bound_addr().port(), 0);
}

#[tokio::test]
async fn bind_server_reports_port_conflict() {
    let first = bind_server(&ServerConfig {
        mode: RuntimeMode::SelfHosted,
        bind_host: "127.0.0.1".to_owned(),
        port: 0,
        ..ServerConfig::default()
    })
    .await
    .expect("first server should bind");

    let conflict = bind_server(&ServerConfig {
        mode: RuntimeMode::SelfHosted,
        bind_host: "127.0.0.1".to_owned(),
        port: first.bound_addr().port(),
        ..ServerConfig::default()
    })
    .await
    .expect_err("second bind should fail");

    assert!(matches!(
        conflict,
        ServerStartError::Bind { source, .. }
            if source.kind() == io::ErrorKind::AddrInUse
    ));
}

#[tokio::test]
async fn bind_server_rejects_remote_only_modes() {
    let error = bind_server(&ServerConfig {
        mode: RuntimeMode::ClientOnly,
        ..ServerConfig::default()
    })
    .await
    .expect_err("client-only should not bind");

    assert!(matches!(error, ServerStartError::LocalServiceUnavailable));
}
