# 平台职责

本文定义正式项目中各平台的职责与公共边界。

## 桌面端

桌面端使用 Tauri v2。

桌面 Shell 负责：

- Tauri 配置；
- 窗口创建；
- WebView 页面地址选择；
- 前端资源打包；
- 桌面权限与操作系统集成；
- 启动和停止共享 Axum 服务；
- 发现本机与局域网访问地址并返回前端；
- 持久化并应用前端请求的运行配置。

启动时，桌面 Shell 必须先注册 Shell Bridge，再加载 Tauri 打包的前端资源；即使配置或 API 服务不可用，前端仍应能打开。
运行设置和服务状态只由前端呈现。Shell 根据前端请求读取、校验、持久化并应用共享配置。
本地模式启动应用时始终在后台启动共享 Axum 服务，不把该行为暴露为可关闭的普通 UI 选项；`client-only` 不启动 Axum。

WebView 打开 Tauri 打包的前端资源，随后前端访问以下 API 根地址之一：

- 本机自用：`http://127.0.0.1:<port>`；
- 连接远端：`remote_base_url`。

桌面 Shell 必须处理：

- 端口冲突提示；
- 优雅关闭；
- 向前端报告服务状态；
- 支持时在配置变化后重启服务；
- 必要的防火墙或系统权限指引；
- 版本化 Shell Bridge 命令、事件和 Tauri capabilities。

桌面 Shell 不得实现原生设置窗口，也不得代理业务 HTTP API。

桌面前端资源由 Tauri 打包，不由 Axum crate 提供。

当前状态：

- 正式 Tauri Shell 尚未实现；
- `desktop/tauri` 尚不存在；
- `desktop/` 下现有普通 Rust 脚手架不是正式桌面架构。

## Android

Android 使用原生 Shell 与 WebView。

Android Shell 负责：

- Activity 生命周期；
- WebView 配置；
- Android 权限；
- native Rust library 加载；
- 启动和停止共享 Axum 服务；
- 前后台运行策略；
- Android 前端与 native 资源打包；
- 发现本机与局域网地址并返回前端；
- 持久化并应用前端请求的运行配置。

启动时，Android Shell 必须先注册 Shell Bridge，再加载 Android 打包的前端资源；即使配置或 API 服务不可用，前端仍应能打开。
运行设置和服务状态只由前端呈现。Shell 根据前端请求读取、校验、持久化并应用共享配置。
本地模式启动应用时始终启动共享 Axum Android native library；`client-only` 不启动 Axum。

WebView 从受信任本地 origin 打开 Android 打包的前端资源，随后前端访问以下 API 根地址之一：

- 本机自用：`http://127.0.0.1:<port>`；
- 连接远端：`remote_base_url`。

Android Shell 必须处理：

- 网络权限声明；
- 使用 HTTP 时的 cleartext policy；
- 生命周期切换；
- 服务关闭；
- 端口冲突提示；
- 需要长期后台运行时的 Foreground Service 要求；
- 限定 origin 的 Shell Bridge 消息与外部导航；
- capability 控制的原生返回请求：同时只有一个 pending、400 ms 超时、页面 generation 失效，以及安全的 WebView/Activity fallback；
- edge-to-edge Window 配置，并把 `systemBars | displayCutout` 发布为 CSS 安全区变量；WebView 铺满 Activity Window，前端按内容语义消费 inset。

Android Shell 不得实现原生设置 Activity 或 Dialog，也不得向不受信任的 WebView origin 暴露 Bridge。

Android 前端资源由 Android 打包，不由 Axum crate 提供。

当前状态：

- 已实现打包 WebView、Shell Bridge、edge-to-edge、受限 origin 的 CSS inset 发布和前端优先的原生返回协商；
- `WineStockApplication` 持有进程级唯一 `LocalCoreRuntimeManager`，Activity 重建不会停止或重建 Rust Runtime 与本地 Axum；
- `android/native` 是唯一 JNI 适配层，复用 `core -> shared`，业务调用仍为 WebView -> HTTP；
- Android `self-hosted` 仅允许 `127.0.0.1`；Foreground Service 与通知策略完成前继续禁用 `server-mode`；
- 当前构建和交付只支持 APK 与 `arm64-v8a`，AAB、32 位 ARM 和 x86 ABI 不属于当前阶段；
- 主机测试、ARM64 交叉构建、Debug/Release APK 构建和包级检查已完成；ARM64 真机已验证 Debug APK
  安装、JNI 加载和本地 `/api/health`，WebView 冷启动恢复、旋转、后台恢复、force-stop 与完整业务 smoke 仍待测试。

## Server

Server Shell 是不带前端的无头进程。

Server Shell 负责：

- 进程生命周期；
- 配置加载；
- 日志与启动状态输出；
- 启动和停止共享 Axum 服务；
- 优雅关闭；
- 展示本机或局域网实际绑定地址。

启动时，Server Shell 读取共享配置。作为纯 API 服务运行时，它使用明确的 `bind_host` 和 `port` 启动共享 Axum 服务；仅暴露自身 API 时不要求配置 `remote_base_url`。

Server Shell 不使用 WebView，也不打包前端资源。

当前状态：

- 正式 Server Shell 位于 `server/`；
- 固定读取或创建可执行文件旁的 `data/config.json`；
- 不接受配置路径参数；
- 使用 JSON 配置启动共享服务、输出访问地址并处理 Ctrl+C 优雅关闭。

## 共享 Axum 服务

桌面端、Android 和 Server Shell 必须调用同一个 Rust 服务库。

服务库提供平台无关能力：

- 创建 Router；
- 启动服务；
- 停止服务；
- 报告实际绑定地址和端口；
- 报告启动和运行错误。

当前实现通过 `start_local_service()` 和 `RunningLocalService` 统一以下生命周期：先绑定端口、再 bootstrap、报告实际地址、观察任务意外退出并执行优雅关闭。Server Shell 与 Android JNI 适配层均复用该 API。

服务库不得感知自身由 Tauri、Android 还是 Server Shell 启动。

## UI 平台的 WebView 契约

所有带 UI 的平台都通过 HTTP 使用服务。

WebView 页面地址指向平台打包的前端资源，不指向 Axum API 根地址。
平台 Shell 根据运行配置和服务启动结果选择 API 根地址，并通过 Shell Bridge 返回前端。
API 根地址不得使用 `0.0.0.0` 或 `::` 作为访问主机。

前端必须在 API 可用性尚未确定时也能加载，并提供配置、启动、重试和失败界面。
平台 Shell 只发布状态并执行生命周期命令，不实现第二套功能 UI。

业务操作继续通过 HTTP；只有运行配置、服务生命周期、实际地址和平台事件使用 Shell Bridge。详见 `docs/shell-bridge.md`。

## 前端资源打包

每个带 UI 的平台使用自己的框架打包前端资源；Server Shell 不打包前端。

允许：

- Tauri 打包桌面前端文件；
- Android assets 打包 Android 前端文件；
- 各平台从同一份 frontend 源码构建自己的产物。

禁止：

- 把平台前端构建产物放入 Axum crate；
- 让 Axum 负责 Tauri 资源服务；
- 让 Axum 负责 Android WebView 资源服务。
