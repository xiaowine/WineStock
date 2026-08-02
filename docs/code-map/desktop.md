# Desktop 代码地图

`desktop` 是 WineStock 正式的 Tauri v2 桌面壳，当前以 Windows 为优先交付目标。它依赖
`core -> shared` 启动本地 Axum 并打包 `frontend/dist`；业务调用仍严格为前端到 HTTP API。

- `desktop/src/`：Tauri 窗口装配、单实例/快捷键等桌面平台插件、受限 Shell Bridge command、版本化 DTO，
  以及进程级 `DesktopRuntimeManager`。`runtime_config.rs` 负责配置校验/持久化，`runtime_snapshot.rs`
  负责状态构造和 core 错误映射；manager 拥有 `RunningLocalService` 的启停、恢复、崩溃快照与退出清理。
  `lan_access.rs` 通过 `if-addrs` 发布 Windows/macOS/Linux 的真实 IPv4 私网访问地址；`firewall.rs` 只在
  Windows 使用高层 `windows` crate 的 Firewall COM 和受限 UAC helper 管理自有规则；
  `webview_compatibility.rs` 在主窗口显示前通过 WebView2 官方 Loader binding 执行 M111
  启动门禁，`webview_privacy.rs` 通过 Tauri 高层配置和 Windows WebView2 Settings 关闭普通表单自动填充与密码自动保存；
  不拥有业务路由、数据库 schema 或前端设置界面。
- `desktop/capabilities/` 与 `permissions/`：只把主窗口绑定到具名 Shell Bridge command 和事件监听，
  不授予通用 shell 或文件系统能力。
- `desktop/tauri.conf.json`、`build.rs`、`icons/`：Tauri 的 Vite 开发/生产资源打包、最小窗口与由
  `brand/` 母版派生的应用图标；不会把前端产物移入 Axum crate。
- `desktop/tests/`：不依赖 WebView 的 runtime manager 集成测试，覆盖首次未配置、local/remote 应用、
  self-hosted 动态端口、server-mode 固定端口/LAN 快照、损坏配置与冷启动恢复。
- `desktop/docs/README.md`：桌面壳的运行、构建和 Windows 验收入口。
