# Android 代码地图

`android` 是 Android 原生 shell，拥有 Activity 生命周期、WebView、打包前端资源加载和 Shell Bridge 传输实现。
它通过 HTTP 使用 core，通过 Shell Bridge 交换运行配置和服务状态，不拥有业务 API、不复制 core 业务实现，也不把业务 UI 托管给 Axum。

当前实现范围是 **Shell Bridge 传输层 + 远端优先**：桥读写运行配置、返回运行快照、推送状态、恢复与原生返回事件，
并支持连接远端服务。端上本地 Axum 尚未实现，本地服务模式返回稳定的 `unsupported_runtime_mode`，
相关 capability 为 `false`。运行配置校验是 `winestock_shared` 规则的 Kotlin 镜像；端上原生 Rust 服务落地后应改为委托 shared。

## 工程入口

- `android/settings.gradle.kts`、`android/build.gradle.kts`：单 `:app` 模块的 Android 工程。
- `android/gradle/libs.versions.toml`：版本目录，含 `androidx.webkit`（Shell Bridge 消息通道与文档起始脚本依赖）。
- `android/app/build.gradle.kts`
  - 声明 `:app` 构建配置、命名空间 `winestock.xiaowine.cc` 和 viewBinding。
  - 从当前 `PATH` 直接执行本机 `pnpm run build:android`，不固定或下载 Node/pnpm 版本，也不读取 `frontend/dist`。
  - 为每个 Android variant 注册前端校验、generated assets 暂存和 APK/AAB 包级验证任务；
    `assemble<Variant>`、`bundle<Variant>` 和 `install<Variant>` 会沿任务图使用当前前端源码。
- `android/buildSrc/src/main/kotlin/winestock/build/FrontendPackagingTasks.kt`
  - 定义前端构建、目录校验、generated assets 暂存、legacy 目录守卫和最终归档验证任务。
  - 构建任务只调用本机 pnpm；依赖未准备、命令失败或产物缺失时立即失败，不执行隐式安装。
- `android/buildSrc/src/main/kotlin/winestock/build/FrontendAssetValidation.kt`
  - 统一校验 Vite 目录与 APK/AAB 内资源的入口、manifest 引用、开发标记、绝对路径和发布 source map。

## Activity 与 WebView

- `android/app/src/main/java/winestock/xiaowine/cc/MainActivity.kt`
  - 唯一 Activity；创建配置 WebView、通过 `WebViewAssetLoader` 从受信任 origin 加载打包前端。
  - 在 `loadUrl` 前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。
  - 保持 edge-to-edge，让 WebView 覆盖完整 Activity Window；不再给根容器统一添加系统栏 padding。
  - 管理与浅色前端一致的系统栏图标、页面可见后的 inset 重发和 SplashScreen；系统返回先交给
    Shell Bridge 协商，未处理、超时或页面未 ready 时才重新判断 WebView history 并交回 dispatcher。
  - 主页面开始加载、Activity pause/stop/destroy 时同步推进或清理原生返回生命周期；`onResume` 恢复协商、
    刷新安全区并通知桥应用恢复。
  - 放开 WebView mixed content，使运行在 `https://winestock.internal` 的前端能连接明文 HTTP 远端服务。
  - 不渲染运行设置或业务 UI，不实现本地 Axum。

- `android/app/src/main/java/winestock/xiaowine/cc/AppConfig.kt`
  - 集中 Android shell 常量：受信任 host `winestock.internal`（ICANN 保留、永不进入公网 DNS）、
    前端入口 URL、Shell Bridge 允许 origin、SplashScreen 超时和原生返回 400ms 应答超时。

- `android/app/src/main/java/winestock/xiaowine/cc/web/FrontendPathHandler.kt`
  - `WebViewAssetLoader.PathHandler`，把受信任 origin 根路径映射到 `assets/frontend`，根路径回退到 `index.html`。
  - 按扩展名推断 MIME 与文本编码；命中失败返回 null 交回默认处理，不做 SPA 回退（前端使用 hash 路由）。

- `android/app/src/main/java/winestock/xiaowine/cc/web/WebViewportInsetsPublisher.kt`
  - 监听根 View 的 `systemBars | displayCutout`，返回原始 WindowInsets，不执行全局 padding 或消费。
  - 按当前 display density 把物理像素换算成 CSS 像素，缓存并去重四边值。
  - 仅在受信任 origin 上通过受控 JavaScript 写入
    `--shell-safe-area-inset-top/right/bottom/left`，页面提交、加载完成或恢复时重发。
  - inset 属于 WebView 渲染环境，不扩展 Shell Bridge v1 业务契约。

- `android/app/src/main/java/winestock/xiaowine/cc/web/LoadingOverlayController.kt`
  - 拥有加载遮罩生命周期：启动兜底超时，收到首个就绪信号后淡出。`hide` 幂等且线程安全。

## Shell Bridge 传输

- `android/app/src/main/assets/shell/bridge.js`
  - 注入 WebView 的传输 shim（属于 Android 平台传输层，不是前端源码）。
  - 构造 `window.__WINESTOCK_SHELL_BRIDGE__`，把 `frontend/src/shell/contract.ts` 的 v1 逻辑接口映射到原生消息通道。
  - 请求信封 `{ type:"call", id, method, params }`，按 id 匹配回复 `{ type:"reply", id, ok, result?, error? }`；
    事件 `{ type:"event", event, payload? }` 驱动 `onRuntimeStateChanged`、`onAppResumed` 和
    `onNativeBackRequested`；`resolveNativeBack` 把前端结算送回 Native。
  - 同时注入 `window.__WINESTOCK_RUNTIME_CONFIG__`（`clientKind:"android"` 与设备/版本元数据）。
  - 消息通道缺失时暴露降级桥，让前端进入可修复失败态。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/ShellBridgeHost.kt`
  - 原生分发：在受信任 origin 上注册 `WebMessageListener` 通道并注入文档起始脚本；对 `WEB_MESSAGE_LISTENER`
    和 `DOCUMENT_START_SCRIPT` 做能力检测。
  - 解析请求信封，路由到配置读取/校验/应用与本地服务生命周期处理，通过 `JavaScriptReplyProxy` 回复。
  - 管理可信主页面代次、ready proxy 和 Activity 可交互状态；发布 `nativeBackRequested`，校验
    `resolveNativeBack` 并把一次性结算交给 broker。
  - 远端模式格式合法即持久化并推送 `configured` 快照；本地模式返回 `unsupported_runtime_mode`，不持久化。
  - `openExternal` 只放行不含凭据的 http/https 并交系统浏览器；`frontendReady` 触发遮罩隐藏回调。
  - 只处理具名平台能力，不代理业务 HTTP、不传递 token、不暴露通用 native 调用。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/NativeBackRequestBroker.kt`
  - 纯状态机地拥有页面代次、单 pending requestId、400ms timeout、重复/迟到应答拒绝和生命周期取消；
    不依赖 Activity、WebView 或 Bridge JSON，因此由 JVM 单元测试覆盖竞态。
- `android/app/src/test/java/winestock/xiaowine/cc/shell/NativeBackRequestBrokerTest.kt`
  - 覆盖 handled、unhandled、timeout、等待期重复返回、duplicate/late resolution、页面换代和 destroy。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeConfig.kt`
  - `EditableRuntimeConfig` 四字段模型与 JSON 序列化、默认配置镜像、`RuntimeModes`、`ShellErrorCodes` 和字段名常量。
  - 字段语义对齐 `contract.ts` 和 `winestock_shared::AppConfig`。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeConfigValidator.kt`
  - `winestock_shared` 校验规则的 Kotlin 镜像：mode 枚举、端口范围、远端 URL 规范化、bindHost IP，
    以及 `normalizeApiBaseUrl`；语义对齐 `web.ts` 与 `api/runtime-config.ts`。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeConfigStore.kt`
  - 版本化 SharedPreferences 持久化，读取分 missing / invalid / present 三态，与 `web.ts` 的 `loadPersistedConfig` 一致。
  - 只保存运行配置，不保存 token 或业务数据。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeSnapshotFactory.kt`
  - 构造 Shell Bridge v1 运行快照，通过 `contract.ts` 的 `assertCompatibleRuntimeSnapshot` 校验：
    `platform:"android"`、六个 capability 布尔字段齐全，远端模式派生 `apiBaseUrl`。
  - `nativeBack` 只随 `ShellBridgeHost.install()` 的真实成功结果开启；页面是否 ready 不改变 capability。

## 资源与配置

- `android/app/src/main/res/layout/activity_main.xml`：四边约束到根容器的全屏 WebView。
- `android/app/src/main/res/values*/colors.xml`、`themes.xml`：Window、SplashScreen 与 WebView 加载前背景；
  当前浅色前端在系统 night mode 下仍保持浅色背景。
- `android/app/src/main/res/xml/network_security_config.xml`：放行明文流量，使远端模式可连接局域网 HTTP 服务器。
- `android/app/src/main/AndroidManifest.xml`：INTERNET 权限、network security config 引用和 Activity 声明。
- `android/app/build/intermediates/winestockFrontend/android/dist`：Vite Android mode 的中间输出，由 Gradle 生成和清理。
- `android/app/build/generated/winestockFrontendAssets/<variant>/frontend`：AGP 对应 variant 消费的已验证 generated assets。

## 启动流程

```text
MainActivity.onCreate
  -> WebViewAssetLoader（域名 winestock.internal，/ -> assets/frontend）
  -> ShellBridgeHost.install（WebMessageListener + document-start shim，限受信任 origin）
  -> WebView.loadUrl(https://winestock.internal/)
  -> 前端读取 window.__WINESTOCK_SHELL_BRIDGE__ 和 __WINESTOCK_RUNTIME_CONFIG__
  -> 前端 getRuntimeSnapshot / applyRuntimeConfig
  -> 前端按 apiBaseUrl 通过 HTTP 使用 core
  -> 前端挂载 nativeBack handler registry 并完成事件订阅
  -> 前端 frontendReady -> 允许原生返回协商并隐藏 SplashScreen
```

## 边界

- 受信任 origin `https://winestock.internal` 仅由 `WebViewAssetLoader` 从本地 assets 提供，不经网络。
- Shell Bridge 消息通道和文档起始脚本都限定该 origin，非受信任 origin 无法调用桥。
- 业务能力通过 HTTP 使用 core；桥只承载运行配置、服务生命周期、真实地址和具名平台事件。
- 端上本地 Axum 与 `server-mode` 前台服务尚未实现，通过对应 capability 关闭；原生返回协商已经实现。
