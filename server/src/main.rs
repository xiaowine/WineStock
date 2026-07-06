#![forbid(unsafe_code)]

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
