//! Desktop runtime manager 的纯逻辑集成测试：验证配置校验、存储与本地 core 全链路。
//!
//! 本模块通过临时目录启动真实 `winestock-core`，不依赖 Tauri 窗口或 WebView。

use std::{
    net::{TcpListener, TcpStream},
    time::Duration,
};

use winestock_desktop::contract::EditableRuntimeConfig;
use winestock_desktop::runtime::DesktopRuntimeManager;

fn temp_manager() -> (tempfile::TempDir, DesktopRuntimeManager) {
    let dir = tempfile::tempdir().expect("temp dir");
    let manager = DesktopRuntimeManager::new(None, dir.path().to_path_buf());
    (dir, manager)
}

fn self_hosted(port: i64) -> EditableRuntimeConfig {
    EditableRuntimeConfig {
        mode: "self-hosted".to_owned(),
        bind_host: "127.0.0.1".to_owned(),
        port,
        remote_base_url: String::new(),
    }
}

fn remote() -> EditableRuntimeConfig {
    EditableRuntimeConfig {
        mode: "client-only".to_owned(),
        bind_host: "127.0.0.1".to_owned(),
        port: 17890,
        remote_base_url: "http://127.0.0.1:18000".to_owned(),
    }
}

fn health_check(api_base_url: &str) -> bool {
    let address = api_base_url
        .replace("http://", "")
        .split('/')
        .next()
        .expect("address")
        .to_owned();
    match TcpStream::connect_timeout(
        &address.parse().expect("socket addr"),
        Duration::from_secs(2),
    ) {
        Ok(mut stream) => {
            use std::io::{Read, Write};
            let _ = stream.write_all(
                b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            );
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            String::from_utf8_lossy(&buffer).contains("OK")
        }
        Err(_) => false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn initial_state_is_unconfigured_and_writes_nothing() {
    let (dir, manager) = temp_manager();
    let snapshot = manager.snapshot().await;
    assert_eq!(snapshot.config_status, "unconfigured");
    assert!(!snapshot.initialized);
    assert_eq!(snapshot.service.phase, "stopped");
    assert!(!dir.path().join("config.json").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn self_hosted_apply_starts_core_and_publishes_loopback_url() {
    let (dir, manager) = temp_manager();
    let result = manager.apply(self_hosted(0)).await;
    assert!(result.applied, "apply 应成功: {:?}", result.error);
    assert_eq!(result.snapshot.service.phase, "running");
    assert_eq!(result.snapshot.service.ownership, "local");
    let api_base_url = result.snapshot.service.api_base_url.expect("api base url");
    assert!(api_base_url.starts_with("http://127.0.0.1:"));
    assert!(
        health_check(&api_base_url),
        "本地服务应可通过 HTTP 健康检查"
    );
    assert!(
        dir.path().join("config.json").exists(),
        "应用成功后应持久化配置"
    );
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("config.json")).unwrap())
            .unwrap();
    assert_eq!(saved["mode"], "self-hosted");
    let persisted_port = saved["port"].as_i64().expect("port");
    assert!(persisted_port > 0, "应持久化实际分配端口");
    manager.shutdown_local_service(Duration::from_secs(5)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn occupied_saved_port_retries_once_with_an_allocated_port() {
    let (_dir, manager) = temp_manager();
    assert!(manager.apply(remote()).await.applied);
    let occupied = TcpListener::bind("127.0.0.1:0").expect("占用测试端口");
    let occupied_port = occupied.local_addr().expect("address").port();

    let result = manager.apply(self_hosted(i64::from(occupied_port))).await;
    assert!(result.applied, "端口冲突应改用动态端口: {:?}", result.error);
    assert_ne!(result.snapshot.config.port, i64::from(occupied_port));
    assert_eq!(result.snapshot.service.phase, "running");

    manager.shutdown_local_service(Duration::from_secs(5)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_apply_persists_without_starting_local_service() {
    let (dir, manager) = temp_manager();
    let result = manager.apply(remote()).await;
    assert!(result.applied);
    assert_eq!(result.snapshot.service.ownership, "remote");
    assert_eq!(result.snapshot.service.phase, "stopped");
    assert_eq!(
        result.snapshot.service.api_base_url.as_deref(),
        Some("http://127.0.0.1:18000")
    );
    assert!(dir.path().join("config.json").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn server_mode_is_rejected() {
    let (_dir, manager) = temp_manager();
    let mut config = self_hosted(17890);
    config.mode = "server-mode".to_owned();
    let result = manager.apply(config).await;
    assert!(!result.valid);
    assert!(result.field_errors.contains_key("mode"));
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_remote_url_is_rejected() {
    let (_dir, manager) = temp_manager();
    let mut config = remote();
    config.remote_base_url = "ftp://example.com".to_owned();
    let result = manager.apply(config).await;
    assert!(!result.valid);
    assert!(result.field_errors.contains_key("remoteBaseUrl"));
}

#[tokio::test(flavor = "multi_thread")]
async fn loaded_config_auto_starts_on_initialize() {
    let (dir, manager) = temp_manager();
    let applied = manager.apply(self_hosted(0)).await;
    assert!(applied.applied);
    let port = applied.snapshot.service.api_base_url.unwrap();
    manager.shutdown_local_service(Duration::from_secs(5)).await;

    // 用同一目录重建 manager，模拟冷启动：有效本地配置应自动恢复服务。
    let manager2 = DesktopRuntimeManager::new(None, dir.path().to_path_buf());
    manager2.initialize().await;
    let snapshot = manager2.snapshot().await;
    assert_eq!(snapshot.service.phase, "running");
    assert_eq!(
        snapshot.service.api_base_url.as_deref(),
        Some(port.as_str())
    );

    manager2
        .shutdown_local_service(Duration::from_secs(5))
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_persisted_config_stays_repairable_without_starting_core() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("config.json"),
        r#"{"mode":"self-hosted","bindHost":"0.0.0.0","port":17890,"remoteBaseUrl":""}"#,
    )
    .expect("write invalid config");

    let manager = DesktopRuntimeManager::new(None, dir.path().to_path_buf());
    manager.initialize().await;
    let snapshot = manager.snapshot().await;
    assert_eq!(snapshot.config_status, "invalid");
    assert!(!snapshot.initialized);
    assert_eq!(snapshot.service.phase, "stopped");
    assert!(snapshot.service.error.is_some());
}
