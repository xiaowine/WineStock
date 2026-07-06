#![forbid(unsafe_code)]

//! WineStock 无头服务端 shell 的二进制入口。
//!
//! 本文件只负责启动异步运行时、调用 `winestock_server::run`，并把错误链打印到 stderr。
//! 配置定位、存储目录准备和 Axum 生命周期编排都在 server shell 库代码中完成。

use std::error::Error;

#[tokio::main]
async fn main() {
    if let Err(error) = winestock_server::run().await {
        eprintln!("WineStock server 启动失败: {error}");
        let mut source = error.source();
        while let Some(cause) = source {
            eprintln!("  caused by: {cause}");
            source = cause.source();
        }
        std::process::exit(1);
    }
}
