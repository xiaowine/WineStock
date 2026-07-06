#![forbid(unsafe_code)]

use std::{env, error::Error, net::SocketAddr};

// 默认使用系统分配的本机端口，避免开发验证时和正式配置端口冲突。
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:0";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned())
        .parse::<SocketAddr>()?;

    // 开发 runner 只验证本机访问，不用于 LAN 暴露或 server-mode 测试。
    if !bind_addr.ip().is_loopback() {
        return Err(format!(
            "dev_server only accepts loopback bind addresses; got {bind_addr}"
        )
        .into());
    }

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let base_url = format!("http://{local_addr}");

    // 打印实际绑定地址，调用方需要使用这个地址访问健康检查和 API 文档。
    println!("WineStock core dev server listening on {base_url}");
    println!("Health: {base_url}/api/health");
    println!(
        "OpenAPI JSON: {base_url}{}",
        winestock_core::OPENAPI_JSON_PATH
    );
    println!(
        "Swagger UI: {base_url}{}",
        winestock_core::SWAGGER_UI_PATH
    );

    axum::serve(listener, winestock_core::build_router()).await?;

    Ok(())
}
