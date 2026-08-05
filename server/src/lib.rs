#![forbid(unsafe_code)]

//! WineStock 无头服务端 shell 的生命周期编排。
//!
//! 本模块属于 `server shell` 层，负责固定配置文件定位、调用 core 统一运行句柄、
//! 启动共享 Axum 服务、打印控制台状态和处理 Ctrl+C 关闭。
//! 它不拥有 API 路由、业务逻辑、桌面/Android UI 或前端打包产物。

mod config;
mod error;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[cfg(debug_assertions)]
use winestock_core::OPENAPI_JSON_PATH;
#[cfg(all(debug_assertions, feature = "swagger-ui"))]
use winestock_core::SWAGGER_UI_PATH;
use winestock_core::{start_local_service, LocalServiceInfo};

pub use error::ServerShellError;

/// 启动无头服务端 shell。
///
/// 配置文件固定为当前可执行文件同目录下的 `data/config.json`。
/// 本函数会创建缺失配置、准备存储目录、初始化 core、绑定 Axum，并阻塞直到收到关闭信号。
pub async fn run() -> Result<(), ServerShellError> {
    let config_path = config::fixed_config_path()?;
    let loaded_config = config::load_config(&config_path)?;
    let config = loaded_config.config;
    config::ensure_server_runtime(&config)?;
    config::prepare_storage_dirs(&config.storage)?;

    let running = start_local_service(&config)
        .await
        .map_err(ServerShellError::LocalService)?;
    let info = running.info();

    print_startup_summary(&config_path, loaded_config.created_default, info);
    shutdown_signal().await;
    running
        .shutdown()
        .await
        .map_err(ServerShellError::LocalService)?;
    println!();
    println!("WineStock Server 已停止。");

    Ok(())
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => println!("收到退出信号，正在关闭服务..."),
        Err(error) => eprintln!("无法监听退出信号: {error}"),
    }
}

/// 打印结构化启动摘要，避免把配置、网络地址和开发文档混在连续日志中。
fn print_startup_summary(
    config_path: &std::path::Path,
    created_default: bool,
    info: &LocalServiceInfo,
) {
    let bound_addr = info.bound_addr;
    let access_url = access_url(bound_addr);

    println!();
    println!("WineStock Server 已启动");
    println!();
    println!("配置");
    println!("  配置文件: {}", config_path.display());
    if created_default {
        println!("  配置状态: 已创建默认配置");
    }
    println!("  数据库: {}", info.database_path.display());
    println!("  文件目录: {}", info.files_dir.display());
    println!();
    println!("服务");
    println!("  监听地址: {}", display_bind_addr(bound_addr));
    println!("  本机访问: {access_url}");
    if info.initial_user_setup_required {
        println!("  初始化状态: 尚未创建首个用户，请通过注册接口完成初始化");
    }
    #[cfg(debug_assertions)]
    {
        println!();
        println!("开发文档");
        println!("  OpenAPI: {access_url}{OPENAPI_JSON_PATH}");
        #[cfg(feature = "swagger-ui")]
        println!("  Swagger UI: {access_url}{SWAGGER_UI_PATH}");
        #[cfg(not(feature = "swagger-ui"))]
        println!("  Swagger UI: 未启用（使用 --features swagger-ui 启动）");
    }
    println!();
    println!("按 Ctrl+C 停止服务。");
}

/// 生成控制台中的监听地址文本。
///
/// 监听地址按实际绑定值原样输出；它与下方可打开的本机访问 URL 是两个概念。
fn display_bind_addr(bound_addr: SocketAddr) -> String {
    format_socket_addr(bound_addr.ip(), bound_addr.port())
}

/// 生成本机可打开的访问 URL。
///
/// 当服务绑定到所有接口时，只给出 loopback URL，避免把 `0.0.0.0` 展示成浏览器地址。
fn access_url(bound_addr: SocketAddr) -> String {
    let port = bound_addr.port();
    let ip = match bound_addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };

    format_url(ip, port)
}

fn format_url(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(ip) => format!("http://{ip}:{port}"),
        IpAddr::V6(ip) => format!("http://[{ip}]:{port}"),
    }
}

fn format_socket_addr(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(ip) => format!("{ip}:{port}"),
        IpAddr::V6(ip) => format!("[{ip}]:{port}"),
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
