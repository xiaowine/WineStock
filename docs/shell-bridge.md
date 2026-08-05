# Shell Bridge 与前端运行配置

本文定义 WineStock UI 平台的 Shell Bridge、运行配置界面、服务生命周期和前端启动边界。
它同时约束 `frontend`、Desktop Tauri shell、Android shell、`core` 和 `shared`，不适用于没有 UI 的 `server` shell。

## 目标

Desktop 和 Android 采用“薄 Shell、单一前端 UI”的结构：

```text
frontend
  -> 唯一用户界面、首次设置、运行配置、服务状态和业务页面

Shell Bridge
  -> 前端配置意图、平台运行状态和生命周期命令

desktop/android shell
  -> WebView、配置持久化、平台路径、core 启停和平台权限

core/shared
  -> 配置模型与校验、Axum 服务、业务能力和持久化
```

Shell 不提供原生设置窗口、Android 设置 Activity、Tauri 原生配置对话框或其它功能性 UI。
系统启动画面和前端加载前的中性占位不属于配置 UI；正常配置错误、端口冲突、服务启动失败和重试操作必须由前端呈现。

## 边界

### HTTP 边界

以下能力始终通过 HTTP 使用 core：

- 鉴权和会话。
- 用户、权限、库存、入库、出库和审计等业务 API。
- 文件上传和下载。
- `/api/health` 服务可用性检查。

Shell Bridge 不得复制 HTTP DTO、代理业务请求或把 Rust 内部业务函数暴露给前端。

### 客户端元数据

`clientKind`、`deviceName` 和 `appVersion` 属于 Shell 启动时提供的客户端元数据，不属于运行快照、可编辑运行配置或业务 API。
UI Shell 必须在前端业务脚本执行前注入 `window.__WINESTOCK_RUNTIME_CONFIG__`；前端只负责读取并将这些值带入登录、注册和本地会话换取请求。

- Desktop：Tauri 初始化脚本在 document-start 阶段注入 `clientKind = "desktop"`、当前系统用户名和 Tauri 包版本；用户名由跨平台实现解析。
- Android：原生 Shell 通过 document-start transport 注入 `clientKind = "android"`、设备型号和应用版本。
- Web：没有原生 Shell 时使用 `VITE_*` 开发配置或 Web 默认值。

Desktop 使用与 Web 相同的 Vite 前端构建；Tauri 只在运行时选择 Desktop transport 并注入客户端元数据，不得把客户端元数据编译进前端产物。

### Shell Bridge 边界

以下能力通过 Shell Bridge：

- 读取前端可编辑的运行配置。
- 校验、保存和应用运行配置。
- 启动、停止和重启本地 Axum。
- 返回真实 API 访问地址、监听地址和稳定错误码。
- 报告应用恢复、原生返回键和服务状态变化。
- 接收前端首屏已渲染的就绪通知。
- 打开经过校验的外部链接等明确平台能力。

桥不得传递 access token、refresh token、密码、数据库连接、Rust 对象或无边界的任意 native method 调用。

## 启动原则

前端资源加载不得依赖 API 服务已经可用。
Shell 必须先加载平台打包的前端资源，再由前端读取配置和运行状态。

Windows Desktop 已有有效本地配置时，可在显示主窗口前完成一次启动恢复，避免把启动过程中的
`configured + stopped` 中间快照误判为首次设置；恢复失败仍必须创建窗口并交给前端呈现运行设置。首次未配置、
远端模式和其它平台不等待本地服务启动。

```text
Shell 启动
  -> 注册 Shell Bridge
  -> 加载平台打包的 frontend/dist
  -> 前端读取 RuntimeSnapshot
  -> initialized=true：Shell 根据持久配置启动本地服务或选择远端地址
  -> initialized=false：保持服务 stopped，由前端展示默认草稿并等待用户选择
  -> 用户 apply 后，Shell 校验、激活并持久化配置
  -> 前端订阅状态并启动 HTTP 健康检查
  -> 前端挂载设置页、错误页或业务路由
  -> 前端发送 frontendReady
```

即使发生以下情况，前端设置页面也必须能够打开：

- 配置文件不存在或无法解析。
- 端口被占用。
- 数据库或文件目录无法创建。
- 数据库打开或迁移失败。
- 远端服务暂时不可访问。
- 本地 Rust 服务未启动。

只有平台打包的前端资源本身无法加载时，才允许退化为平台日志或系统级致命错误处理。

## 前端可编辑配置

前端不直接读写平台配置文件，也不应默认暴露平台绝对存储路径。
桥向前端提供专门的可编辑 DTO：

```ts
interface EditableRuntimeConfig {
  mode: "self-hosted" | "client-only" | "connect-to-remote" | "server-mode";
  bindHost: string;
  port: number;
  remoteBaseUrl: string;
}
```

`port` 在 `self-hosted` 下允许临时值 `0`，表示请求 Shell 自动分配端口；绑定成功后 Shell 必须把实际端口回写到配置和运行快照。`running` 快照和持久化配置中的端口必须始终为 `1..65535`。`server-mode` 始终要求用户配置固定端口。

字段语义必须映射到 `winestock_shared::AppConfig`，但桥 DTO 可以使用适合 TypeScript 的命名并由平台适配层转换。
配置文件仍由各平台 Shell 决定位置，并使用 shared 的模型和校验作为权威结果。
`auto_start_server` 不是 UI 配置项：本地模式在配置已 initialized 后固定映射为 `true` 并在应用启动时运行服务；
首次缺少配置时仍需等待前端 apply，远端模式不启动本地服务。

### 地址区分

必须区分：

- `bindHost`：Axum 监听地址。
- `remoteBaseUrl`：远端客户端模式的目标服务地址。
- `apiBaseUrl`：Shell 计算并返回给前端的实际 HTTP API 根地址。

本地模式下，前端不能自行根据 `bindHost` 或草稿端口拼接 `apiBaseUrl`，尤其不能生成 `http://127.0.0.1:0`。
例如 `bindHost = 0.0.0.0` 时，本机前端仍应使用 `http://127.0.0.1:<port>`，不能访问或展示 `http://0.0.0.0:<port>`。

只有 `client-only` 和 `connect-to-remote` 远端客户端模式允许用户直接设置远端 API 地址。
如果产品界面暂时只展示一个“连接远端服务”选项，前端和桥必须定义稳定的规范化规则，不能在读取现有配置后丢失模式语义。

## 运行快照

Shell 向前端返回配置和当前生效状态的统一快照：

```ts
interface RuntimeSnapshot {
  protocolVersion: 1;
  platform: "web" | "desktop" | "android";
  configStatus: "configured" | "unconfigured" | "invalid";
  config: EditableRuntimeConfig;
  initialized: boolean;
  service: {
    ownership: "local" | "remote";
    phase: "stopped" | "starting" | "running" | "stopping" | "failed";
    apiBaseUrl?: string;
    boundAddress?: string;
    localAuthExchangeToken?: string;
    lanAccessUrls?: string[];
    error?: ShellRuntimeError;
  };
  capabilities: {
    startLocalService: boolean;
    stopLocalService: boolean;
    restartLocalService: boolean;
    nativeBack: boolean;
    openExternal: boolean;
    serverMode: boolean;
  };
}
```

`initialized` 是 Shell 对运行配置初始化状态的权威判断：

- `false`：没有可用的持久配置，或保存记录已经损坏；Shell 可以返回默认表单草稿，但不得据此启动本地 HTTP 服务。
- `true`：交互式 Shell 的配置已经由用户成功应用并持久化，或 Web 部署提供了明确的权威地址；
  后续冷启动按该配置自动启动本地服务或选择远端地址。
- 它不表示 core 当前健康。已初始化配置仍可能对应 `starting`、`failed` 或暂时不可达的远端服务。

前端不得根据 `configStatus`、`apiBaseUrl`、默认端口或本地存储自行推断 initialized。设置漏斗完成条件为
`initialized && configured && apiBaseUrl`；服务错误仍由设置/恢复界面处理。

`lanAccessUrls` 只用于 Shell 管理的本地 `server-mode` 服务。Shell 必须根据当前真实网络接口和
实际监听端口返回可供其它设备输入的 HTTP/HTTPS 根地址，并在服务或网络状态变化时发布新快照。
该字段不得包含 `0.0.0.0`、`::`、loopback、用户名密码、业务路径或占位字符串；没有真实地址时应省略
或返回空数组，不能返回 `http://<局域网地址>:port` 一类伪值。前端可以做防御性过滤和去重，但不得
自行枚举网卡、根据 `bindHost` 拼接地址或改变 Shell 给出的接口优先顺序。

`localAuthExchangeToken` 是 `self-hosted` 本地服务每次启动生成的本机会话换取凭据（见
`docs/implementation-notes/self-hosted-silent-auth.md`）：仅 `ownership=local` 的快照允许携带，
由 core 经进程内接口交给 Shell、再经受信任通道交给前端，用于换取正常登录 token 实现本机免登录。
Shell 不得把它写入日志或持久化存储；Web 桥永远不提供该字段。

`capabilities` 是前端判断平台能力的唯一依据。
前端不得根据 `window.AndroidBridge`、`window.__TAURI__`、User-Agent 或目录结构猜测功能。

## Shell Bridge v1

前端统一依赖以下逻辑接口，Android 和 Tauri 只负责提供不同传输实现：

```ts
interface ShellBridge {
  getRuntimeSnapshot(): Promise<RuntimeSnapshot>;
  validateRuntimeConfig(
    config: EditableRuntimeConfig,
  ): Promise<RuntimeConfigValidationResult>;
  applyRuntimeConfig(
    config: EditableRuntimeConfig,
  ): Promise<ApplyRuntimeConfigResult>;
  startLocalService(): Promise<RuntimeSnapshot>;
  stopLocalService(): Promise<RuntimeSnapshot>;
  restartLocalService(): Promise<RuntimeSnapshot>;
  repairFirewall?(): Promise<RuntimeSnapshot>;
  frontendReady(): Promise<void>;
  reportFrontendFailure?(message: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  setWindowTheme?(theme: "system" | "light" | "dark"): Promise<void>;
  getDesktopPreferences?(): Promise<DesktopPreferences>;
  setDesktopPreferences?(preferences: DesktopPreferences): Promise<DesktopPreferences>;
  onRuntimeStateChanged(
    listener: (snapshot: RuntimeSnapshot) => void,
  ): Promise<() => void>;
  onAppResumed(listener: () => void): Promise<() => void>;
  onNativeBackRequested?(
    listener: (request: NativeBackRequest) => void,
  ): Promise<() => void>;
  resolveNativeBack?(
    resolution: NativeBackResolution,
  ): Promise<{ accepted: boolean }>;
}
```

### 平台更新扩展

Desktop 和 Android 各自提供更新检查与安装扩展；Web 不提供该能力。更新清单请求、下载安装包、完整性校验和平台安装器均留在对应 Shell，前端不直接访问更新域名，避免 WebView 跨域和文件权限边界泄漏。

```ts
interface AppUpdateCheckResult {
  currentVersion: string;
  latestVersion?: string;
  notes?: string;
}

interface AppUpdateShellBridgeExtension {
  checkForUpdate(): Promise<AppUpdateCheckResult>;
  installUpdate(version: string): Promise<void>;
}
```

`installUpdate` 只接收前端已经展示的版本号。Shell 必须在安装前重新请求自身平台的更新清单并确认版本仍可用，不能信任前端传入的下载地址。检查失败、清单无效、下载失败、摘要校验失败和 Android 安装权限缺失都必须通过稳定错误码返回，由前端显示可恢复提示；错误不得只写入原生日志，也不得把原始网络异常、文件路径或堆栈暴露给前端。

当前清单地址由平台固定配置：

- Android：`https://api.ikuns.top/WineRealm/file/winestock/android.json`
- Desktop：`https://api.ikuns.top/WineRealm/file/winestock/desktop.json`

更新比较使用语义版本。Android 更新清单不包含 `versionCode`，但 APK 内部的 Android `versionCode` 仍必须递增，且更新 APK 必须保持相同 `applicationId` 和签名证书。

Desktop 可选扩展只承载当前设备的窗口、自启动偏好和 Windows 防火墙操作，不属于运行配置或业务 API：

```ts
type DesktopCloseBehavior = "minimize-to-tray" | "exit-application";

interface DesktopPreferences {
  version: 1;
  closeBehavior: DesktopCloseBehavior;
  autostartEnabled: boolean;
  autostartSilent: boolean;
  webviewReclaimEnabled: boolean;
  webviewReclaimIdleMinutes: 5 | 15 | 30 | 60 | 120 | 240;
}
```

只有 `platform = "desktop"` 的 Tauri bridge 提供 Desktop 扩展方法；Web/Android 不需要实现，前端在未提供扩展时隐藏对应能力。
Desktop 的 `setWindowTheme` 也属于该扩展：前端主题运行时通过桥同步三态偏好，Windows Desktop Shell 才调用 Tauri 原生窗口主题 API，macOS/Linux 实现保持 no-op。
Desktop shell 将偏好保存在自己的 app data 文件并缓存到进程状态。`autostartEnabled` 返回系统启动项的实际状态，
由 Tauri autostart 插件的 `is_enabled()` 校准；`autostartSilent` 是本机持久化偏好。`CloseRequested` 只读取缓存，
不在窗口事件中查询磁盘或等待 WebView IPC。自启动进程通过内部 `--winestock-autostart` 参数与普通手动启动区分，
静默启动只对带该参数且托盘可用的进程生效。`webviewReclaimEnabled` 和
`webviewReclaimIdleMinutes` 控制托盘隐藏后的主 WebView 空闲回收；回收只销毁 WebView，保留 Tauri 进程、托盘和
本地 Axum 服务。WebView 重新创建后重新执行前端 Shell Bridge 握手和 `frontendReady`。

`nativeBack` 是 v1 内 capability-gated 的可选扩展，不要求普通 Web fallback 或旧平台桥实现：

- `capabilities.nativeBack = false` 时，前端不得调用两个可选方法；
- `capabilities.nativeBack = true` 时，两个方法必须同时存在，否则按 `invalid_bridge_payload` 处理；
- Android 发送 `nativeBackRequested { requestId, canGoBack }`，前端以
  `resolveNativeBack { requestId, handled, reason }` 结算；`accepted = false` 表示请求已经超时、取消、结算或属于旧页面；
- `requestId` 由 Native 生成并携带页面代次；`reason` 只用于诊断，Android 只依据 `handled` 决定是否 fallback；
- 前端必须先安装订阅，再调用 `frontendReady()`。页面刷新、Activity pause 或销毁会取消 pending，且不额外执行 fallback。

普通浏览器和 Vite 开发环境必须提供 Web fallback。
Web fallback 可以只支持读取环境变量、返回 `platform = web` 和 no-op 生命周期能力，不得让本地开发依赖原生桥。

桥协议必须携带版本号。
不兼容版本应返回 `bridge_version_mismatch`，不能静默按旧结构解释数据。

## 配置校验与应用

前端可以先做即时表单校验，但平台 Shell 必须使用 shared 配置模型执行权威校验。
校验错误应返回稳定字段路径和错误码，供前端映射到表单：

```ts
interface RuntimeConfigFieldError {
  field: "mode" | "bindHost" | "port" | "remoteBaseUrl";
  code: string;
  message: string;
}
```

### 本地模式应用流程

本地配置采用“验证并激活成功后提交”的策略：

```text
前端提交草稿
  -> Shell/shared 权威校验
  -> 记录旧配置和旧运行状态
  -> 停止需要替换的旧服务
  -> 使用新配置准备存储并启动 core
  -> 绑定成功后计算实际 apiBaseUrl
  -> 持久化新配置并发布 running 快照
```

如果新服务启动失败：

- 不把失败草稿写成新的正式配置。
- 有旧配置时尽力恢复旧服务和旧快照。
- 没有旧配置时保持 `unconfigured` 或 `failed`，并让前端保留草稿。
- 返回端口、配置、存储、迁移或服务错误，不弹原生对话框。

固定端口被占用时，Shell 仅对 `self-hosted` 自动使用端口 `0` 重试一次；绑定成功后先用实际端口更新配置，再持久化并发布 `running` 快照。`server-mode` 保持固定端口错误路径。

存储路径或数据库迁移可能产生不可逆的外部副作用；未来允许前端编辑存储位置时，必须增加单独确认和迁移策略，不能把它当作普通地址设置。

### 远端模式应用流程

远端 URL 格式校验失败时禁止保存。
远端服务暂时无法连接不应阻止保存，因为目标服务可能只是离线。
保存后返回 `remote` 快照，由前端 HTTP 健康检查呈现“配置有效但当前不可连接”。

## 前端运行状态

前端应维护独立于鉴权和 HTTP 健康检查的 Shell 启动状态：

```ts
type RuntimeBootstrapStatus =
  | "loading"
  | "unconfigured"
  | "config-invalid"
  | "starting"
  | "running"
  | "remote-unavailable"
  | "failed";
```

推荐呈现：

```text
loading                 -> 前端自身的中性启动屏
unconfigured/invalid    -> 首次设置或运行设置页
starting                -> 本地服务启动状态
running                 -> 正常业务路由
remote-unavailable      -> 远端断连状态，保留设置入口
failed                  -> 精确运行错误，保留修改和重试入口
```

设置路由不得依赖 API 或鉴权，例如：

```text
/setup
/settings/runtime
```

可以使用 `requiresService = false`、`requiresAuth = false` 等路由元数据保证服务完全不可用时仍能进入设置。
全局服务不可用覆盖层不能遮挡这些路由。

## API client 重配置

当前生效 `apiBaseUrl` 发生变化时，前端必须按顺序；但 Desktop 本地 `server-mode` 仅切换端口时，
服务身份没有变化，不清理当前登录会话：

1. 暂停健康检查和自动刷新。
2. 取消仍在进行的旧服务请求。
3. 远端切换或运行服务切换时清理内存 access token 和旧服务会话状态；同一 Desktop 本地 server-mode 端口变化保留它们。
4. 使用新的 `apiBaseUrl` 重配置 API client。
5. 恢复 `/api/health` 检查。
6. 对新地址重新初始化鉴权会话；同一 Desktop 本地 server-mode 端口变化继续使用当前会话。
7. 根据结果进入登录页、原业务路由或运行设置页。

refresh token 必须继续绑定 API 根地址，切换服务时不得把旧服务 token 发送到新服务。Desktop 本地
server-mode 仅端口变化时，Shell 确认新旧快照均为同一 local server-mode 后，前端可以把 refresh token
的绑定地址从旧本机端口迁移到新本机端口；不得对远端服务切换复用该例外。
API client 应支持显式的 `unconfigured` 状态，不能因为缺少地址就在模块导入阶段阻止整个 Vue 应用挂载。

## Shell 运行职责

Desktop 和 Android Shell 应：

- 在前端加载前注册桥传输。
- 决定配置文件、数据库和文件目录的实际平台路径。
- 调用 shared 加载和校验配置；首次缺失时只向前端提供默认草稿。
- 调用 core 启动、停止和查询本地服务。
- 生成本机 loopback 和实际 LAN 访问地址。
- 持久化成功激活的配置。
- 向前端发布版本化快照和稳定错误码。
- 在平台退出时优雅关闭本地服务。

如果前端无法完成 Shell Bridge 契约校验、原生扩展订阅或首屏握手，前端不得继续挂载业务界面，
应通过可选的 `reportFrontendFailure` 上报给当前平台 Shell。Android 销毁 WebView 并复用原生兼容性阻断页；
Desktop 隐藏 WebView、通过 `rfd` 显示平台错误提示后退出。该路径属于 Shell/加载失败，不是运行配置或业务错误。

Shell 不应：

- 渲染运行设置表单或业务错误对话框。
- 根据错误文案驱动前端分支。
- 读取或持久化浏览器鉴权 token。
- 复制 core 路由、业务 DTO 或业务校验。

## Core 服务句柄

core 应逐步收敛出平台无关的运行句柄，使 Desktop 和 Android 不重复拼装 bootstrap、bind、serve 和 shutdown：

```rust
pub struct RunningLocalService {
    // 实际 API 只需表达绑定地址、关闭和等待停止；具体字段可以调整。
}

pub async fn start_local_service(
    config: &AppConfig,
) -> Result<RunningLocalService, LocalServiceRuntimeError>;
```

平台 Shell 决定何时调用 start/stop；core 不监听 Activity、窗口关闭、Ctrl+C 或系统托盘事件。

## Android 约束

Android 应使用平台打包资源加载前端，不把业务 UI 托管给 Axum。
推荐由 Gradle 构建任务生成 Android assets，并通过受信任的本地 WebView origin 加载。
生成的 `frontend/dist` 不应手工复制或作为普通源码重复维护。

正式桥应优先使用支持 origin 限制的 AndroidX WebKit 消息能力。
如果暂时使用 `addJavascriptInterface`，必须同时满足：

- WebView 只加载受信任的打包前端。
- 导航到非受信任 origin 前移除或禁用桥。
- 外部 URL 交给系统浏览器。
- 不向桥暴露敏感数据或通用反射调用。

Android `self-hosted` 服务属于应用进程，不应因为 Activity 旋转或短暂后台切换立即停止。
需要在后台持续供其他设备访问的 `server-mode` 必须明确 Foreground Service、通知和系统限制；未实现前通过 capability 禁用该模式。

Android 使用打包 HTTPS origin 调用本地或远端 HTTP API 时，必须显式处理 WebView mixed-content 和 Android cleartext policy。
默认优先支持 loopback 自托管和 HTTPS 远端服务，不得无提示地扩大明文网络范围。

Android 原生返回由生命周期感知的 `OnBackPressedDispatcher` 作为唯一入口。Activity 同时最多维护一个请求，
等待期间的重复返回直接消费、不排队；前端优先关闭临时浮层、页面内步骤或调用 Vue Router 返回。
前端返回 `handled = false` 或 400ms 未应答时，Activity 必须重新读取 `WebView.canGoBack()`：可返回则调用
`WebView.goBack()`，否则临时禁用当前 callback 并把返回交回 dispatcher。刷新、pause 和 destroy 只取消旧请求，
避免应用已离开前台后由迟到 timeout 再触发返回。

## Desktop Tauri v2 约束

Tauri 使用平台打包的前端资源，不把主窗口指向 Axum API 地址。
前端通过 `@tauri-apps/api` 的 `invoke` 和事件 API 实现 Shell Bridge 适配层。

Tauri 只注册具名命令，例如：

```text
shell_get_runtime_snapshot
shell_validate_runtime_config
shell_apply_runtime_config
shell_start_local_service
shell_stop_local_service
shell_restart_local_service
shell_repair_firewall
shell_get_desktop_preferences
shell_set_desktop_preferences
shell_frontend_ready
```

capabilities 只允许主窗口调用必要命令，不能为方便启用宽泛 shell 执行权限。
Desktop 窗口关闭行为由本机偏好决定：选择最小化到托盘时拦截关闭并隐藏窗口，选择退出应用时进入同一套
`ExitRequested` 清理流程。托盘明确退出和系统退出都必须等待本地 Axum 优雅停止后再结束进程。
Desktop 偏好还支持由官方 Tauri autostart 插件管理的开机自启和自启动静默；插件状态由 Shell command
封装，前端不得直接调用 autostart 插件 API。自启动注册失败使用稳定的 Desktop 偏好错误返回，不能让前端显示
未生效的成功状态。开启 WebView 回收后，窗口隐藏超过偏好时长才销毁主 WebView；此过程必须阻止由最后一个窗口
销毁引起的 `ExitRequested`，不得调用 `DesktopRuntimeManager::shutdown_local_service`。

## 前端资源与 API 地址

WebView 页面地址与 API 地址是两个不同概念：

```text
WebView 页面地址 -> Tauri/Android 平台打包资源
API 地址         -> http://127.0.0.1:<port> 或 remoteBaseUrl
```

Axum 不服务 Desktop 或 Android 前端构建产物。
平台打包资源必须能够在离线且 API 未启动时打开运行设置页。

## 默认配置与首次启动

UI 平台缺少持久配置时不得自动创建配置文件，也不得启动本地 Axum HTTP 服务。
Shell 返回 `initialized=false`、`configStatus=unconfigured`、`service.phase=stopped`，并把 shared 默认配置
（本机自托管、loopback、默认端口 `17890`）仅作为前端表单草稿。

用户在前端选择模式并提交 `applyRuntimeConfig` 后：

- 本地模式先使用候选配置启动 core，成功后持久化实际配置并发布 `initialized=true`；
- 远端模式校验并持久化地址，然后发布 `initialized=true`，且不启动本地 HTTP 服务；
- 首次应用任一步骤失败都保持 `initialized=false`；已有配置的变更失败则恢复旧配置及其初始化状态，
  不把失败草稿伪装成正式配置。

已有有效持久配置的后续冷启动仍自动激活，不重复展示首次选择；Activity/窗口重建和 WebView reload 复用
Shell 的进程级运行状态，不重复启动服务。

## 稳定错误码

第一版至少定义：

```text
bridge_version_mismatch
invalid_bridge_payload
config_unavailable
config_invalid
storage_unavailable
database_open_failed
migration_failed
invalid_bind_host
port_in_use
service_start_failed
service_crashed
native_library_unavailable
unsupported_runtime_mode
```

`message` 用于用户展示，前端分支只判断稳定 `code` 和字段路径，不解析文案。
平台日志可以记录底层错误链，但桥返回内容不能泄漏敏感路径、凭据或内部调试信息。

## 实施顺序

建议分阶段实施：

1. 定义 Shell Bridge v1 DTO、错误码和 Web fallback。
2. 让前端在没有 API 地址时仍能挂载运行设置页。
3. 使 API client、健康检查和鉴权会话支持地址切换。
4. 在 core 增加可停止的本地服务运行句柄。
5. 完成 Android 打包资源、受控桥传输和本地 Rust 服务。
6. 建立正式 Tauri v2 shell 并复用相同前端接口。
7. 再按真实需要增加配置迁移、LAN 地址、server-mode、系统托盘或自动更新。

第一版不要加入原生设置 UI、任意 native invoke、业务 API 代理或无业务依据的兼容层。

## 验收

实现完成后至少验证：

- 无配置文件时前端可进入运行设置，Shell 返回 `initialized=false` 且不启动本地 HTTP 服务、不写配置。
- 首次应用本地配置成功后才启动 core、写入配置并发布 `initialized=true`。
- 首次应用远端配置后不启动本地 core，后续冷启动直接恢复远端模式。
- 配置损坏时仍能打开运行设置并修复。
- 本地服务正常启动并返回真实 loopback API 地址。
- 端口占用时设置页显示稳定错误并可修改重试。
- 远端地址无法连接时配置仍可保存，设置页保持可用。
- 切换 API 地址后不会把旧服务 token 发送到新服务。
- 服务重启期间当前设置草稿和页面上下文不丢失。
- Android Activity 重建不会错误停止应用级本地服务。
- Desktop 退出会等待 Axum 优雅关闭并释放端口。
- 平台离线启动时能够加载打包前端和运行设置。
- 非受信任 WebView origin 无法调用 Android 桥。
- Tauri capabilities 只开放 Shell Bridge 所需命令。
