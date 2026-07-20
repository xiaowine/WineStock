# Android Shell 文档

本目录是 WineStock Android 平台 shell 的文档入口。
Android shell 负责 Activity 生命周期、WebView、前端资源打包加载和 Shell Bridge 传输，不拥有 core 业务规则、前端页面或运行设置 UI。

## Shell Bridge 实现

Android 实现 [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md) 定义的 Shell Bridge v1 契约，作为前端 `window.__WINESTOCK_SHELL_BRIDGE__` 的一个注入传输。
契约字段、错误码、快照结构和边界以该文档为权威，本目录不重复定义。

传输实现要点：

- 通过 `androidx.webkit` 的 `WebMessageListener` + document-start 脚本，在受信任 origin `https://winestock.internal` 上建立 JS ↔ Kotlin 消息通道。
- 前端资源由 `WebViewAssetLoader` 从 `assets/frontend` 提供，Gradle `syncFrontendAssets` 任务从 `frontend/dist` 同步。
- 信封协议、方法路由和快照派生写在代码注释中（`shell/bridge.js`、`shell/ShellBridgeHost.kt`）。

## 当前边界与未实现

- 端上本地 Axum 尚未实现：本地模式返回 `unsupported_runtime_mode`，`startLocalService` 等能力为 `false`；远端模式为当前主用法。
- 运行配置校验是 shared 规则的 Kotlin 镜像（对齐前端 web fallback）；端上原生 Rust 服务落地后应改为委托 `winestock_shared`。
- 为连接局域网明文 HTTP 服务器，已放行 cleartext 与 WebView mixed-content，范围见 `network_security_config.xml` 与代码注释。

## 相关文档

- [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md)：Shell Bridge v1 契约与边界（权威）。
- [`../../docs/code-map/android.md`](../../docs/code-map/android.md)：Android shell 源码结构。
- [`../../docs/platforms.md`](../../docs/platforms.md)：Android 平台职责。
