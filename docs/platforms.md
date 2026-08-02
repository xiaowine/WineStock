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
已有 initialized 本地配置时，应用启动会先恢复共享 Axum 服务，再创建并显示主窗口，避免前端把短暂的
`configured + stopped` 快照误判为需要进入运行设置；该行为不作为可关闭的普通 UI 选项；
首次缺少配置时等待前端 apply，`client-only` 不启动 Axum。
主窗口创建后保持隐藏，正常情况下收到前端 `frontendReady` 首帧信号后才显示，避免 WebView 加载过程闪烁；
8 秒未收到信号时由 Desktop Shell 受控显示窗口，避免桥或前端异常导致窗口永久隐藏。

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

- 正式 Tauri v2 Shell 位于 `desktop`，是 Cargo 工作区成员并优先交付 Windows；
- 主窗口加载 Tauri 打包的 `frontend/dist`，通过具名 command/event 提供 Shell Bridge v1；
- `DesktopRuntimeManager` 在 app data 目录管理配置、SQLite/文件路径与 `RunningLocalService`，首次无配置保持 stopped，
  有效本地配置在主窗口显示前恢复，local/remote 切换与退出均停止旧服务；
- Desktop 的 LAN 地址发现应使用跨平台高层接口，防火墙由各平台独立 provider 管理；Windows 采用高层
  `windows` crate 的 Firewall COM，macOS 可使用 PF、Linux 可使用 firewalld/nftables，但必须分别验证权限、
  规则所有权和回滚，不得假设三者防火墙策略一致；
- `tauri-plugin-single-instance` 保证桌面进程单实例；后续启动只聚焦首个主窗口，不向其转交参数、工作目录或 URL，
  后续实例随后退出；`tauri-plugin-prevent-default` 仅在 Release 禁用 WebView2 默认快捷键，Debug 保留默认快捷键；
- Desktop 主窗口支持在偏好设置中选择“最小化到系统托盘”或“退出应用”；默认最小化到托盘。托盘隐藏只隐藏窗口，
  不停止本地 core；托盘“退出”以及选择直接退出后的窗口关闭，统一等待本地 Axum 优雅停止。托盘不可用时采用直接退出的安全降级，
  不把窗口隐藏到无法恢复的状态；
- Desktop 偏好支持由 `tauri-plugin-autostart` 管理的“开机自启”和“静默启动”。系统启动项状态由 Shell 查询，
  前端只通过 Shell Bridge 读写；静默启动仅对带内部自启动标记的进程生效，手动启动和单实例恢复仍显示窗口，
  托盘不可用时按显示窗口的 fail-safe 处理；
- Windows Desktop 在显示主窗口前调用 WebView2 官方 Loader API 检查 Evergreen Runtime 主版本不低于 M111；不满足时不加载
  前端、不启动本地服务，通过 `rfd` 原生错误对话框提示依赖损坏并要求重新安装软件后退出；安装器同步使用 `minimumWebview2Version=111.0.0.0`。macOS/Linux
  使用系统 WebKit，待接入对应原生版本 API 后再启用同一门禁。
- Windows Desktop 已开放 `server-mode`；macOS/Linux 是否开放由 capability 和防火墙 provider 能力决定，不能把
  `serverMode=true` 解释为自动放行；Windows 安装包 smoke 尚待执行。

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
首次没有持久配置时，Shell 发布 `initialized=false` 和默认草稿，不创建配置文件、不启动本地 Axum HTTP 服务；
用户在前端选择模式并成功应用后才启动本地 core 或切换远端。已有有效本地配置的后续冷启动仍自动启动
共享 Axum 服务；`client-only` 不启动 Axum。

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
- `LocalCoreRuntimeManager` 只自动激活持久配置；首次未初始化保持 stopped，前端 apply 成功后才启动/选择服务；
- `android/native` 是唯一 JNI 适配层，复用 `core -> shared`，业务调用仍为 WebView -> HTTP；
- Android `self-hosted` 仅允许 `127.0.0.1`；Foreground Service 与通知策略完成前继续禁用 `server-mode`；
- 当前构建和交付只支持 APK 与 `arm64-v8a`，AAB、32 位 ARM 和 x86 ABI 不属于当前阶段；
- 主机测试、ARM64 交叉构建、Debug/Release APK 构建和包级检查已完成；API 33 ARM64 真机已验证
  Debug APK 安装、JNI 加载、离线冷启动、远端/本机 HTTP、原有旋转、后台恢复、force-stop 恢复和原生返回
  浮层/路由 smoke。当前 Activity 已锁定 `sensorPortrait`，禁止切换横屏，新增锁定规则仍待真机复验；首次未初始化不启服的新漏斗仍待真机复验；其它 Android 版本、手势导航、异常注入和完整业务矩阵仍待覆盖。

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
