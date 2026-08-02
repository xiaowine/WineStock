//! WineStock Desktop（Tauri v2）正式壳库入口。
//!
//! 二进制与集成测试共享 Shell Bridge、配置持久化与本地 Axum 生命周期实现。

pub mod commands;
pub mod contract;
pub mod firewall;
pub(crate) mod lan_access;
pub mod runtime;
pub mod webview_compatibility;
pub mod webview_privacy;
