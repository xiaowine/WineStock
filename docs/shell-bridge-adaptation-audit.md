# Shell Bridge 适配审计与后续计划

本文记录当前 Shell Bridge v1 在共享前端、Web fallback、Desktop Tauri 和 Android shell 中的适配情况，
并区分“应该通过 Shell Bridge 的运行控制”和“有意保持为平台独立实现的 WebView/系统能力”。
审计依据为 `frontend/src/shell/contract.ts`、`docs/shell-bridge.md` 及当前四套传输实现。

## 结论摘要

当前 v1 运行控制链路已经打通：

- 前端统一通过 `frontend/src/shell/runtime.ts` 使用 Shell Bridge，不直接调用 Rust、JNI 或 Android API。
- Desktop 使用 Tauri command/event，Android 使用受信任 origin 的 WebMessageListener，普通浏览器使用 Web fallback。
- 运行配置、初始化状态、本地 Core 启停、真实 API 地址、错误快照、外链和应用恢复事件均有对应实现。
- Android 原生返回已按 capability-gated 扩展接入；Desktop 和 Web 明确关闭该能力。

本轮已完成的整改包括：

1. 统一 Tauri invoke 失败的稳定错误码解析；
2. 让 Desktop/Web 的 capability 快照反映当前运行模式；
3. 增加 Shell Bridge 快照语义、错误传输和 Desktop lifecycle 的测试入口；
4. Desktop 使用 `frontendReady` 正常显示路径，并增加 8 秒隐藏窗口兜底。

仍待后续评估的是客户端元数据 v2，以及 Android/真实 Tauri 传输层的设备级 smoke 覆盖。

## 当前结构

```text
frontend/src/shell/contract.ts   v1 逻辑契约、快照校验、能力判断
        │
        ├── frontend/src/shell/transports/tauri.ts
        │      └── Tauri invoke/listen
        │             └── desktop/src/commands.rs
        │                    └── DesktopRuntimeManager
        │
        ├── android/app/src/main/assets/shell/android-transport.js
        │      └── WebMessageListener 信封
        │             └── ShellBridgeHost.kt
        │                    └── LocalCoreRuntimeManager
        │
        └── frontend/src/shell/transports/web.ts
               └── 浏览器 localStorage、环境变量和 Web API fallback
```

Shell Bridge 只负责运行配置、服务生命周期、运行快照、平台事件和明确的外链能力；业务 API、鉴权 token、
数据库对象、WebView 对象和任意 native invoke 都不属于桥。

## v1 方法适配矩阵

| v1 能力                   | 前端统一入口                    | Web fallback                  | Desktop Tauri                                                    | Android shell                                                  | 状态             |
|-------------------------|---------------------------|-------------------------------|------------------------------------------------------------------|----------------------------------------------------------------|----------------|
| `getRuntimeSnapshot`    | `runtime.ts` 初始化          | 从环境变量/localStorage 构造快照       | `shell_get_runtime_snapshot` → `DesktopRuntimeManager::snapshot` | `getRuntimeSnapshot` → `LocalCoreRuntimeManager`               | 已适配            |
| `validateRuntimeConfig` | 运行设置/向导调用                 | 浏览器字段校验                       | `shell_validate_runtime_config` → shared/core 规则                 | `validateRuntimeConfig` → native/shared；native 不可用时远端 fallback | 已适配            |
| `applyRuntimeConfig`    | 统一 apply 流程并更新 API client | 保存 localStorage，浏览器不启动本地服务    | 停旧服务、启动 core、保存配置、发布快照                                           | JNI/core 事务、实际端口回写、SharedPreferences 保存                        | 已适配            |
| `startLocalService`     | 服务恢复页重试                   | 返回 unsupported，不暴露 capability | command → manager 启动本地服务                                         | command → manager/JNI 启动本地服务                                   | 已适配；能力已动态收敛    |
| `stopLocalService`      | 本地服务控制                    | 返回 unsupported                | command → manager 停止服务                                           | command → manager/JNI 停止服务                                     | 已适配；能力已动态收敛    |
| `restartLocalService`   | 本地服务故障恢复                  | 返回 unsupported                | command → manager 重启服务                                           | command → manager/JNI 重启服务                                     | 已适配；能力已动态收敛    |
| `frontendReady`         | `main.ts` 首帧后调用           | no-op                         | 显示隐藏的 Tauri 主窗口                                                  | 解除 Android Splash，并锁定当前页面代次/返回代理                               | 已适配；设备级 smoke 待补 |
| `openExternal`          | 设置页/隐私链接调用                | `window.open`                 | `tauri-plugin-opener`                                            | ACTION_VIEW 系统浏览器                                              | 已适配            |
| `onRuntimeStateChanged` | 更新快照、API 地址和服务可用性         | 内存 listener                   | Tauri event `winestock-runtime-state-changed`                    | WebMessage `runtimeStateChanged`                               | 已适配            |
| `onAppResumed`          | 取消旧健康检查并重新探测              | `visibilitychange`            | 主窗口重新获得焦点                                                        | Activity resume 后发送                                            | 已适配            |
| `onNativeBackRequested` | 原生返回 registry             | capability=false              | capability=false                                                 | WebMessage 事件，单 pending/400ms/页面代次                             | 已适配 Android 扩展 |
| `resolveNativeBack`     | 结算返回请求                    | capability=false              | capability=false                                                 | WebMessage 应答，迟到/重复返回 `accepted=false`                         | 已适配 Android 扩展 |

### 当前刻意关闭或未投影的字段

- `service.lanAccessUrls`：Desktop 仅在真实 `server-mode` 本地服务运行时发布真实 IPv4 私网地址；Android 和 Web
  fallback 不生成地址，不能用 `0.0.0.0` 或伪造地址填充该字段。
- `capabilities.serverMode`：Desktop 为 `true`；Android 和 Web fallback 为 `false`。纯 Web 页面仍保留远端连接配置入口。
- `capabilities.nativeBack`：只有 Android 在桥安装成功后开放；Desktop 不应通过 Tauri 自行模拟 Android 返回协议。
- `normalizedConfig`：Rust/Kotlin 内部校验结果可以携带规范化配置，但当前前端 v1 只依赖 `valid` 和 `fieldErrors`；
  若要把规范化结果纳入公开契约，应先同步 TypeScript、Tauri、Android 和 Web 三侧，而不是单边增加字段。

## 已有的独立实现

以下能力没有使用 Shell Bridge，但根据当前职责划分属于正确的独立平台实现，不应为了“统一”而改成任意桥方法。

### Desktop/Tauri 独立能力

- `desktop/src/webview_compatibility.rs`：调用 WebView2 Loader 官方 API 做 M111 版本门禁。
- `desktop/src/main.rs`：窗口创建、隐藏/显示、单实例聚焦、退出时等待 Core 关闭。
- `tauri-plugin-single-instance`：第二实例聚焦首实例并退出，不向桥转发参数或 URL。
- `tauri-plugin-prevent-default`：Release 禁用 WebView2 默认快捷键，Debug 保留调试快捷键。
- `tauri-plugin-opener`：前端 Tauri transport 直接调用 `openUrl`；主窗口 capability 只允许项目 GitHub 页、QQ群链接和 Microsoft 隐私声明，插件本身不属于桥协议。
- Tauri 资源打包、CSP、窗口尺寸和 NSIS 安装器：属于平台打包，不通过前端桥控制。
- Desktop 前端客户端元数据当前由 `vite.config.ts` 的 desktop build-time define 提供，不是运行时桥字段。

### Android 独立能力

- `WebViewCompatibility` 和兼容性页面：在创建 WebView 前检查 provider 版本及 WebKit capability。
- `WebViewAssetLoader`、`ShellWebViewConfigurator` 和 `MainShellCoordinator`：加载受信任打包资源、处理页面/渲染器生命周期。
- `SplashFrontendGate`：以 `frontendReady` 为主、`onPageFinished` 和超时为兜底，控制 Android Splash。
- `WebViewportInsetsPublisher`：将系统栏、挖孔和 IME inset 转成 CSS 安全区变量；这不是运行控制协议。
- `SystemBarAppearanceController` 与前端 `window.WineStockSystemChrome`：系统栏图标明暗和主题表面控制。
- `WebViewFileChooserHost`、`WebViewCameraPermissionHost`：HTML 文件选择和摄像头权限属于 WebChromeClient/ActivityResult。
- `NativeCoreClient`、JNI 和 `android/native`：仅是 Android shell 到共享 core 的内部适配，不向前端暴露业务 native API。
- `window.__WINESTOCK_RUNTIME_CONFIG__`：Android 注入客户端类型、设备名和版本号，供登录/遥测元数据使用；它不是运行配置快照。

### Web fallback 独立能力

- `localStorage` 保存浏览器运行配置；浏览器不能启动本地 Axum，因此本地生命周期 capability 为 false。
- `window.open` 是浏览器外链 fallback。
- `visibilitychange` 负责浏览器恢复事件。

### 始终不应进入 Shell Bridge

- 业务 HTTP API、鉴权 access/refresh token、数据库/文件对象。
- WebView provider 查询、窗口/Activity 生命周期、系统栏和输入法布局。
- 摄像头、文件选择器、系统返回 fallback、安装器和操作系统权限。
- Android JNI 业务调用或 Tauri 任意命令执行。

## 需要继续适配或收敛的项目

### 已完成：统一 Desktop invoke 错误码

整改前 Android shim 的 `toBridgeError` 会把 `{ code, message }` 还原为带 `code` 的 Error；Tauri 传输则直接返回
`invoke` Promise，`commands.rs` 把错误 JSON 放在 `String` 中，调用失败时前端拿到的通常是普通字符串错误。

这会导致 `start/stop/restart/openExternal` 的 command 失败不能稳定按 `port_in_use`、
`service_start_failed` 等错误码分支，和 `docs/shell-bridge.md` 的稳定错误码要求不完全一致。

实现：`frontend/src/shell/transports/bridgeError.ts` 提供统一规范化函数，`transports/tauri.ts` 的所有 command invoke 都经过该函数，
兼容 Tauri rejection 的字符串、对象和 Error 三种形状，输出带稳定 `code` 的 `ShellBridgeTransportError`。
页面组件不再各自解析错误文案。

### 已完成：收敛 capability 语义

Desktop `desktop_capabilities()` 固定把本地 start/stop/restart 设为 `true`，即使当前快照是远端模式、
首次未配置或服务不属于本地。后端 command 会再次拒绝不适用的调用，因此目前不会越权，但它与 Android
的动态 `localLifecycleAvailable` 不一致。

实现：Desktop `desktop_capabilities(initialized, ownership)` 仅在已初始化的本地配置上开放本地生命周期；Web
fallback 已将 `serverMode` 改为 `false`，与浏览器不能托管 Core 的事实一致。

### 已完成第一阶段：建立跨传输契约测试

新增 `frontend/tests/shellBridgeContract.test.mjs` 和 `test:shell-bridge-contract`，覆盖三种传输共用的快照语义、
Tauri 错误 JSON 规范化和不安全地址拒绝；Desktop manager 测试同时覆盖动态 capability。仍可在未来补充真实设备
WebMessage/installed Tauri smoke，但不再缺少共享契约的纯逻辑门槛。

后续设备级 smoke 仍建议覆盖：

- `RuntimeSnapshot` 的字段/枚举/能力组合；
- 端口冲突重试、首次未初始化、配置损坏、远端模式和服务崩溃；
- `frontendReady`、状态事件和恢复事件的页面代次/订阅生命周期；
- 稳定错误码在 Tauri、Android 和 Web fallback 中的相同语义。

实现方式：以 `frontend/src/shell/contract.ts` 为断言入口，平台传输继续只负责投影 JSON；平台具体测试不复制
core 业务测试。

### 已完成：明确首屏就绪的故障兜底

Desktop 当前创建隐藏窗口，正常情况下由前端 `frontendReady` command 显示；这能避免 WebView 首屏闪烁，异常时
由受控超时兜底显示。Android 已有 `onPageFinished + timeout` Splash 兜底。

实现：Desktop 主窗口保持隐藏，`frontendReady` 正常路径显示；`desktop/src/main.rs` 增加 8 秒受控超时，
前端脚本或桥初始化异常时仍显示可恢复的设置/错误页。Android 原有 `onPageFinished + timeout` 逻辑保持不变。

### P2：统一客户端元数据的来源

Android 使用运行时注入的 `__WINESTOCK_RUNTIME_CONFIG__`，Desktop 使用 Vite build-time define，Web 使用
环境变量。三者最终都由 `resolveApiClientMetadata()` 消费，但版本号和设备名的更新时机不同。

适配方式：若未来需要运行时更新版本/设备名，新增明确的 v2 `getClientMetadata` 或把元数据纳入快照；不要在
现有 v1 中临时增加未声明字段。当前 build-time/runtime 双路径可以保留。

### 已完成：补充快照语义校验

实现：`assertCompatibleRuntimeSnapshot` 现在额外校验 API 地址安全性、initialized/configStatus 组合、running
地址、本地换取凭据归属及本地 lifecycle capability 组合，违规统一归类为 `invalid_bridge_payload`。

## 不建议的“适配”方向

- 不把 WebView2/Android WebView 版本查询塞进 `getRuntimeSnapshot`；这是创建 WebView 前的启动门禁。
- 不把安全区、系统栏、主题、文件选择和摄像头权限扩展成任意 Shell command；这些是平台 UI 能力。
- 不让 Shell Bridge 代理业务 HTTP 或传递鉴权 token。
- 不为 Desktop/Android 各自维护第二套运行设置 UI。
- 不因为 Tauri、Android 和 Web 的传输不同，就复制三份运行配置/服务生命周期业务逻辑；权威规则继续放在
  `shared`、`core` 和各自 manager 的平台边界内。

## 建议实施顺序

1. 已完成 Tauri 错误码解析、Desktop/Web capability 收敛、共享快照语义测试和 Desktop 首屏超时兜底。
2. 下一步补 Android WebMessage 与真实 Tauri 安装包 smoke，验证传输层生命周期和错误投影。
3. 若确有运行时版本/设备名更新需求，再设计 v2 客户端元数据能力。
4. Desktop `server-mode` 代码已完成；剩余安装包和真实网络 smoke 后，再考虑 Desktop 原生返回或其它新
   capability；每项都必须先扩展协议、权限和测试矩阵。

## 验证入口

| 范围                   | 当前验证                                                                                                                    |
|----------------------|-------------------------------------------------------------------------------------------------------------------------|
| 前端桥与运行漏斗             | `cd frontend && pnpm run test:shell-bridge-contract`、`pnpm run test:runtime-funnel`、`pnpm run test:availability-policy` |
| Desktop manager/契约   | `cargo test -p winestock-desktop -- --test-threads=1`                                                                   |
| Rust 格式              | `cargo +stable fmt --all -- --check`                                                                                    |
| Android manager/桥状态机 | `android/` Gradle JVM 单测与 `LocalCoreRuntimeManagerTest`                                                                 |
| 平台 smoke             | Desktop 首次设置、已有配置冷启动、Core 失败恢复、隐藏窗口首帧显示；Android WebView 兼容性、旋转、后台恢复、原生返回                                                |

当前文档只记录适配审计和计划，不改变 Shell Bridge v1 协议，也不新增代码兼容层。
