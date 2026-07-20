# Android 代码地图

`android` 是 Android 原生 shell，拥有 Activity 生命周期、WebView、打包前端资源加载和 Shell Bridge 传输实现。
它通过 HTTP 使用 core，通过 Shell Bridge 交换运行配置和服务状态，不拥有业务 API、不复制 core 业务实现，也不把业务 UI 托管给 Axum。

当前实现范围是 **Shell Bridge 传输层 + 远端优先**：桥读写运行配置、返回运行快照、推送状态与恢复事件，
并支持连接远端服务。端上本地 Axum 尚未实现，本地服务模式返回稳定的 `unsupported_runtime_mode`，
相关 capability 为 `false`。运行配置校验是 `winestock_shared` 规则的 Kotlin 镜像；端上原生 Rust 服务落地后应改为委托 shared。

## 工程入口

- `android/settings.gradle.kts`、`android/build.gradle.kts`：单 `:app` 模块的 Android 工程。
- `android/gradle/libs.versions.toml`：版本目录，含 `androidx.webkit`（Shell Bridge 消息通道与文档起始脚本依赖）。
- `android/app/build.gradle.kts`
  - 声明 `:app` 构建配置、命名空间 `winestock.xiaowine.cc` 和 viewBinding。
  - `syncFrontendAssets`（`Sync` 任务）把 `frontend/dist` 同步到 `app/src/main/assets/frontend`，挂在 `preBuild` 前；
    `dist` 缺失时禁用任务以避免误删已打包资源。生成目录不纳入版本库，需先在 `frontend/` 执行 `pnpm build`。

## Activity 与 WebView

- `android/app/src/main/java/winestock/xiaowine/cc/MainActivity.kt`
  - 唯一 Activity；创建配置 WebView、通过 `WebViewAssetLoader` 从受信任 origin 加载打包前端。
  - 在 `loadUrl` 前安装 Shell Bridge，保证 document-start 脚本先于页面脚本注入。
  - 管理系统栏外观与 inset、加载遮罩、WebView 返回键；`onResume` 通知桥应用恢复。
  - 放开 WebView mixed content，使运行在 `https://winestock.internal` 的前端能连接明文 HTTP 远端服务。
  - 不渲染运行设置或业务 UI，不实现本地 Axum。

- `android/app/src/main/java/winestock/xiaowine/cc/AppConfig.kt`
  - 集中 Android shell 常量：受信任 host `winestock.internal`（ICANN 保留、永不进入公网 DNS）、
    前端入口 URL、Shell Bridge 允许 origin 和加载遮罩超时/淡出时长。

- `android/app/src/main/java/winestock/xiaowine/cc/web/FrontendPathHandler.kt`
  - `WebViewAssetLoader.PathHandler`，把受信任 origin 根路径映射到 `assets/frontend`，根路径回退到 `index.html`。
  - 按扩展名推断 MIME 与文本编码；命中失败返回 null 交回默认处理，不做 SPA 回退（前端使用 hash 路由）。

- `android/app/src/main/java/winestock/xiaowine/cc/web/LoadingOverlayController.kt`
  - 拥有加载遮罩生命周期：启动兜底超时，收到首个就绪信号后淡出。`hide` 幂等且线程安全。

## Shell Bridge 传输

- `android/app/src/main/assets/shell/bridge.js`
  - 注入 WebView 的传输 shim（属于 Android 平台传输层，不是前端源码）。
  - 构造 `window.__WINESTOCK_SHELL_BRIDGE__`，把 `frontend/src/shell/contract.ts` 的 v1 逻辑接口映射到原生消息通道。
  - 请求信封 `{ type:"call", id, method, params }`，按 id 匹配回复 `{ type:"reply", id, ok, result?, error? }`；
    事件 `{ type:"event", event, payload? }` 驱动 `onRuntimeStateChanged` 和 `onAppResumed`。
  - 同时注入 `window.__WINESTOCK_RUNTIME_CONFIG__`（`clientKind:"android"` 与设备/版本元数据）。
  - 消息通道缺失时暴露降级桥，让前端进入可修复失败态。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/ShellBridgeHost.kt`
  - 原生分发：在受信任 origin 上注册 `WebMessageListener` 通道并注入文档起始脚本；对 `WEB_MESSAGE_LISTENER`
    和 `DOCUMENT_START_SCRIPT` 做能力检测。
  - 解析请求信封，路由到配置读取/校验/应用与本地服务生命周期处理，通过 `JavaScriptReplyProxy` 回复。
  - 远端模式格式合法即持久化并推送 `configured` 快照；本地模式返回 `unsupported_runtime_mode`，不持久化。
  - `openExternal` 只放行不含凭据的 http/https 并交系统浏览器；`frontendReady` 触发遮罩隐藏回调。
  - 只处理运行配置与服务生命周期，不代理业务 HTTP、不传递 token、不暴露通用 native 调用。

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

## 资源与配置

- `android/app/src/main/res/layout/activity_main.xml`：WebView 与加载遮罩布局。
- `android/app/src/main/res/xml/network_security_config.xml`：放行明文流量，使远端模式可连接局域网 HTTP 服务器。
- `android/app/src/main/AndroidManifest.xml`：INTERNET 权限、network security config 引用和 Activity 声明。

## 启动流程

```text
MainActivity.onCreate
  -> WebViewAssetLoader（域名 winestock.internal，/ -> assets/frontend）
  -> ShellBridgeHost.install（WebMessageListener + document-start shim，限受信任 origin）
  -> WebView.loadUrl(https://winestock.internal/)
  -> 前端读取 window.__WINESTOCK_SHELL_BRIDGE__ 和 __WINESTOCK_RUNTIME_CONFIG__
  -> 前端 getRuntimeSnapshot / applyRuntimeConfig
  -> 前端按 apiBaseUrl 通过 HTTP 使用 core
  -> 前端 frontendReady -> 隐藏加载遮罩
```

## 边界

- 受信任 origin `https://winestock.internal` 仅由 `WebViewAssetLoader` 从本地 assets 提供，不经网络。
- Shell Bridge 消息通道和文档起始脚本都限定该 origin，非受信任 origin 无法调用桥。
- 业务能力通过 HTTP 使用 core；桥只承载运行配置、服务生命周期、真实地址和平台事件。
- 端上本地 Axum、`server-mode` 前台服务和原生返回键协商尚未实现，通过 capability 关闭。
