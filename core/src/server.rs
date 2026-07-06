//! core 的 Axum 网络绑定和服务运行封装。
//!
//! 本模块只负责把共享 `ServerConfig` 转换成已绑定的 TCP listener，
//! 并提供带关闭信号的 Axum serve 入口。用户可见 URL 展示和进程生命周期属于平台 shell。

use std::{
    error::Error,
    fmt,
    future::Future,
    io,
    net::{AddrParseError, IpAddr, SocketAddr},
};

use tokio::net::TcpListener;
use winestock_shared::ServerConfig;

/// 已完成端口绑定、等待平台壳启动服务的 Axum 实例。
#[derive(Debug)]
pub struct BoundServer {
    listener: TcpListener,
    bound_addr: SocketAddr,
}

impl BoundServer {
    /// 返回操作系统实际绑定的地址，端口为 0 时这里会包含真实端口。
    pub fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    /// 使用平台壳提供的关闭信号启动 Axum 服务。
    pub async fn serve_with_shutdown<S>(self, shutdown: S) -> Result<(), ServerStartError>
    where
        S: Future<Output = ()> + Send + 'static,
    {
        axum::serve(self.listener, crate::build_router())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(ServerStartError::Serve)
    }
}

/// Axum 服务绑定或运行失败。
#[derive(Debug)]
pub enum ServerStartError {
    /// 当前运行模式不需要本地 Axum 服务，不能执行绑定。
    LocalServiceUnavailable,

    /// `server.bind_host` 不是合法 IP 地址。
    InvalidBindHost {
        /// 配置中的原始绑定地址。
        host: String,

        /// IP 解析失败原因。
        source: AddrParseError,
    },

    /// TCP listener 绑定失败，通常是端口占用或权限问题。
    Bind {
        /// 绑定失败的目标地址。
        addr: SocketAddr,

        /// 底层 IO 错误。
        source: io::Error,
    },

    /// 绑定成功后读取实际监听地址失败。
    LocalAddr(io::Error),

    /// Axum 服务运行过程中返回错误。
    Serve(io::Error),
}

impl fmt::Display for ServerStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalServiceUnavailable => write!(
                f,
                "current runtime mode does not start a local Axum service"
            ),
            Self::InvalidBindHost { host, .. } => {
                write!(f, "invalid server.bind_host value: {host}")
            }
            Self::Bind { addr, source } if source.kind() == io::ErrorKind::AddrInUse => {
                write!(f, "port is already in use: {addr}")
            }
            Self::Bind { addr, .. } => write!(f, "failed to bind Axum service at {addr}"),
            Self::LocalAddr(_) => write!(f, "failed to read bound Axum address"),
            Self::Serve(_) => write!(f, "Axum service stopped with an error"),
        }
    }
}

impl Error for ServerStartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidBindHost { source, .. } => Some(source),
            Self::Bind { source, .. } => Some(source),
            Self::LocalAddr(source) | Self::Serve(source) => Some(source),
            Self::LocalServiceUnavailable => None,
        }
    }
}

/// 根据共享配置完成网络绑定，平台壳负责决定何时调用和如何展示状态。
pub async fn bind_server(config: &ServerConfig) -> Result<BoundServer, ServerStartError> {
    if !config.uses_local_service() {
        return Err(ServerStartError::LocalServiceUnavailable);
    }

    let bind_ip =
        config
            .bind_host
            .parse::<IpAddr>()
            .map_err(|source| ServerStartError::InvalidBindHost {
                host: config.bind_host.clone(),
                source,
            })?;
    let requested_addr = SocketAddr::new(bind_ip, config.port);
    let listener =
        TcpListener::bind(requested_addr)
            .await
            .map_err(|source| ServerStartError::Bind {
                addr: requested_addr,
                source,
            })?;
    let bound_addr = listener.local_addr().map_err(ServerStartError::LocalAddr)?;

    Ok(BoundServer {
        listener,
        bound_addr,
    })
}

#[cfg(test)]
mod tests {
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
}
