#![forbid(unsafe_code)]

mod config;
mod error;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use winestock_core::{bind_server, bootstrap_from_config, OPENAPI_JSON_PATH, SWAGGER_UI_PATH};

pub use error::ServerShellError;

pub async fn run() -> Result<(), ServerShellError> {
    let config_path = config::fixed_config_path()?;
    let loaded_config = config::load_config(&config_path)?;
    let config = loaded_config.config;
    config::ensure_server_runtime(&config)?;
    config::prepare_storage_dirs(&config.storage)?;

    let bootstrap = bootstrap_from_config(&config)
        .await
        .map_err(ServerShellError::CoreBootstrap)?;
    let local = bootstrap
        .local_service
        .as_ref()
        .ok_or(ServerShellError::LocalServiceNotInitialized)?;

    println!("WineStock server 配置文件: {}", config_path.display());
    if loaded_config.created_default {
        println!("已创建默认配置文件: {}", config_path.display());
    }
    println!("数据库: {}", local.storage.database_path.display());
    println!("文件目录: {}", local.storage.files_dir.display());
    if local.auth.admin_setup_required {
        println!("首次管理员尚未初始化；请通过后续 setup 流程创建管理员。");
    }

    let bound = bind_server(&config.server)
        .await
        .map_err(ServerShellError::Start)?;
    let bound_addr = bound.bound_addr();
    let access_url = access_url(bound_addr);
    println!("监听地址: {}", display_bind_addr(bound_addr));
    println!("访问地址: {access_url}");
    println!("健康检查: {access_url}/api/health");
    println!("OpenAPI JSON: {access_url}{OPENAPI_JSON_PATH}");
    println!("Swagger UI: {access_url}{SWAGGER_UI_PATH}");

    println!("按 Ctrl+C 停止服务。");
    bound
        .serve_with_shutdown(shutdown_signal())
        .await
        .map_err(ServerShellError::Start)?;
    println!("WineStock server 已停止。");

    Ok(())
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => println!("收到退出信号，正在关闭服务..."),
        Err(error) => eprintln!("无法监听退出信号: {error}"),
    }
}

fn display_bind_addr(bound_addr: SocketAddr) -> String {
    let port = bound_addr.port();
    match bound_addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => format!("所有 IPv4 接口:{port}"),
        IpAddr::V6(ip) if ip.is_unspecified() => format!("所有 IPv6 接口:{port}"),
        ip => format_socket_addr(ip, port),
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn access_url_never_uses_unspecified_ipv4_address() {
        let url = access_url(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 17890)));

        assert_eq!(url, "http://127.0.0.1:17890");
    }

    #[test]
    fn explicit_access_url_is_reported_directly() {
        let url = access_url(SocketAddr::from((Ipv4Addr::new(10, 0, 0, 8), 17890)));

        assert_eq!(url, "http://10.0.0.8:17890");
    }
}
