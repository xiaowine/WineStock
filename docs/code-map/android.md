# Android 代码地图

`android` 是 Android 原生 shell，拥有 Activity 生命周期、WebView、打包前端资源加载和 Shell Bridge 传输实现。
它通过 HTTP 使用 core，通过 Shell Bridge 交换运行配置和服务状态，不拥有业务 API、不复制 core 业务实现，也不把业务 UI 托管给 Axum。

当前实现范围是 **打包前端 + Application 级本地 core + 远端降级**：Android 默认可在进程内启动共享
Axum/core，桥读写运行配置、返回运行快照、异步管理服务并保留恢复与原生返回事件。权威配置校验来自
`android/native -> winestock_shared`；native library 不可用时仍能打开设置页并连接远端。

## 工程入口

- `android/settings.gradle.kts`、`android/build.gradle.kts`：单 `:app` 模块的 Android 工程。
- `android/gradle/libs.versions.toml`：版本目录，含 `androidx.webkit`（Shell Bridge 消息通道与文档起始脚本依赖）。
- `android/app/build.gradle.kts`
  - 声明 `:app` 构建配置、命名空间 `winestock.xiaowine.cc` 和 viewBinding。
  - 从当前 `PATH` 直接执行本机 `pnpm run build:android`，不固定或下载 Node/pnpm 版本，也不读取 `frontend/dist`。
  - 固定 NDK `30.0.14904198` 与唯一 ABI `arm64-v8a`，注册 variant-aware Rust JNI 构建、ELF 校验、
    generated jniLibs 和 APK 包级验证。
  - 为每个 Android variant 注册前端校验、generated assets 暂存和 APK 包级验证；当前没有 AAB
    校验或 bundle 挂钩，交付物只支持 APK。
- `android/buildSrc/src/main/kotlin/winestock/build/FrontendPackagingTasks.kt`
  - 定义前端构建、目录校验、generated assets 暂存、legacy 目录守卫和最终归档验证任务。
  - 构建任务只调用本机 pnpm；依赖未准备、命令失败或产物缺失时立即失败，不执行隐式安装。
- `android/buildSrc/src/main/kotlin/winestock/build/FrontendAssetValidation.kt`
  - 统一校验 Vite 目录与 APK 内资源的入口、manifest 引用、开发标记、绝对路径和发布 source map。

- `android/buildSrc/src/main/kotlin/winestock/build/RustNativePackagingTasks.kt`
  - `RustNativeBuildTask` 以 `cargo-ndk 4.1.2`、`--locked --offline` 构建 Debug 或 Release ARM64 `.so`；
    Release 使用 Cargo `--release`，因此 core/shared 传递依赖也按 release profile 编译。
  - `RustNativeVerifyTask` 检查 ELF64/AArch64、8 个具名 JNI 导出、允许的系统动态库和 profile marker。
  - `RustNativeApkVerifyTask` 检查最终 APK 只含 `lib/arm64-v8a/libwinestock_android_native.so`。

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
  - 从 `WineStockApplication` 取得进程级 manager，只绑定 Bridge/WebView，不拥有或停止本地 Axum。
  - 通过 `WebChromeClient.onShowFileChooser` + Activity Result 启动系统文件选择器（`FileChooserParams.createIntent`
    / SAF），把选定的 `content://` URI 交回 WebView；不申请存储/媒体/相机权限。

- `android/app/src/main/java/winestock/xiaowine/cc/WineStockApplication.kt`
  - 在 Application 创建时初始化唯一 `LocalCoreRuntimeManager`；Activity 重建不替换 core runtime。

## Application 级 core 与 JNI

- `android/app/src/main/java/winestock/xiaowine/cc/core/LocalCoreRuntimeManager.kt`
  - 单线程后台 executor 串行执行 JNI、配置校验、启动/停止/重启和 SharedPreferences 提交。
  - 首次 `self-hosted` 使用端口 `0` 请求系统分配；绑定成功后把 native 返回的实际端口写回并持久化。
  - 已保存端口冲突时仅对 `self-hosted` 自动重试一次动态端口；`server-mode` 保持固定端口错误路径。
  - 候选配置先激活后提交，启动或保存失败时恢复旧服务；持有权威快照并向 Bridge 订阅者推送，
    Activity 生命周期不触发 shutdown。
- `core/NativeLibraryLoader.kt`、`NativeCoreClient.kt`、`NativeCoreBridge.kt`
  - 安全、幂等加载 `libwinestock_android_native.so`，封装具名静态 JNI 方法和 native protocol v1 调用。
  - load/JNI 失败映射 `native_library_unavailable`，不在类初始化时让应用崩溃。
- `core/NativeContract.kt`
  - 编解码 native v1 请求/响应、字段错误、规范化配置和实际服务地址；拒绝协议版本不匹配。
  - 校验 `running` 状态的 `boundAddress` 与 `apiBaseUrl` 使用同一非零实际端口，
    防止把临时端口 `0` 发布给前端。
- `core/AndroidStoragePaths.kt`
  - 固定使用 `noBackupFilesDir/winestock/data`，预创建数据库父目录与文件仓目录。

- `android/native/`
  - Cargo `cdylib`/测试 crate，是 Android 唯一 Rust 适配层；依赖 `winestock-core` 和 `winestock-shared`。
  - `ffi.rs` 使用 jni-rs 0.22 `EnvUnowned.with_env()`，每个 JNI 入口只交换 JSON 并阻止 panic 越过 FFI。
  - `engine.rs` 持有双 worker Tokio Runtime 和 `RunningLocalService`，提供 start/stop/restart/state。
  - `config.rs` 使用 shared 权威校验并额外限制 Android self-hosted 仅 `127.0.0.1`、禁用 server-mode；
    `self-hosted + port=0` 作为临时自动分配请求传给 core，运行响应始终返回实际绑定地址。

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

- `android/app/src/main/java/winestock/xiaowine/cc/web/WebViewFileChooserSession.kt`
  - 纯状态机地拥有 HTML 文件选择的单 pending `ValueCallback`、supersede 时 null 结算、
    cancel/destroy 一次结算，以及 ClipData/单 URI/取消的结果映射。
  - 与 MainActivity 单个 ActivityResultLauncher 配套：supersede 后的唯一结果结算新 pending，
    不用 stale 丢弃计数（避免把二次选择结果当过期丢掉导致 WebView 挂起）。
  - 不启动 Intent、不依赖 WebView，不做 `content://` 路径反查；由 JVM 单元测试覆盖竞态。
- `android/app/src/test/java/winestock/xiaowine/cc/web/WebViewFileChooserSessionTest.kt`
  - 覆盖 deliver、cancel、空结果、supersede 后单次 deliver 结算新回调、destroy 与 mapChooserResult。

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
  - 解析请求信封，把配置与本地服务生命周期异步路由到 Application manager，通过主线程
    `JavaScriptReplyProxy` 回复；页面 generation 变化后丢弃迟到结果。
  - 管理可信主页面代次、ready proxy 和 Activity 可交互状态；发布 `nativeBackRequested`，校验
    `resolveNativeBack` 并把一次性结算交给 broker。
  - manager 发布的本地/远端快照会转成 `runtimeStateChanged`；Bridge 不复制配置事务或 JNI 逻辑。
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

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RemoteRuntimeConfigFallbackValidator.kt`
  - 仅在 native 不可用时校验/规范化远端 http(s) 地址，不校验本地模式，不作为 shared 规则镜像。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeConfigStore.kt`
  - 版本化 SharedPreferences 持久化，读取分 missing / invalid / present 三态，与 `web.ts` 的 `loadPersistedConfig` 一致。
  - 只保存运行配置，不保存 token 或业务数据。

- `android/app/src/main/java/winestock/xiaowine/cc/shell/RuntimeSnapshotFactory.kt`
  - 把 manager 状态构造为 Shell Bridge v1 快照；本地生命周期 capability 只在 native 可用且当前
    ownership 为 local 时开启，`serverMode` 固定 false。
- `shell/AsyncBridgeReplyGate.kt`
  - 独立保存页面 generation；刷新、导航或 destroy 后拒绝旧异步调用的迟到回复。

## 资源与配置

- `android/app/src/main/res/layout/activity_main.xml`：四边约束到根容器的全屏 WebView。
- `android/app/src/main/res/values*/colors.xml`、`themes.xml`：Window、SplashScreen 与 WebView 加载前背景；
  当前浅色前端在系统 night mode 下仍保持浅色背景。
- `android/app/src/main/res/xml/network_security_config.xml`：放行明文流量，使远端模式可连接局域网 HTTP 服务器。
- `android/app/src/main/AndroidManifest.xml`：仅 `INTERNET` 权限、network security config 引用和 Activity 声明；
  WebView 文件选择走系统选择器，不声明存储/媒体/相机/全文件访问权限。
- `android/app/build/intermediates/winestockFrontend/android/dist`：Vite Android mode 的中间输出，由 Gradle 生成和清理。
- `android/app/build/generated/winestockFrontendAssets/<variant>/frontend`：AGP 对应 variant 消费的已验证 generated assets。

## 启动流程

```text
MainActivity.onCreate
  -> WineStockApplication.LocalCoreRuntimeManager（进程级异步初始化/默认本地 core）
  -> WebViewAssetLoader（域名 winestock.internal，/ -> assets/frontend）
  -> ShellBridgeHost.install（WebMessageListener + document-start shim，限受信任 origin）
  -> WebView.loadUrl(https://winestock.internal/)
  -> 前端读取 window.__WINESTOCK_SHELL_BRIDGE__ 和 __WINESTOCK_RUNTIME_CONFIG__
  -> 前端 getRuntimeSnapshot / applyRuntimeConfig
     -> ShellBridgeHost 异步调用 LocalCoreRuntimeManager
     -> JNI -> android/native -> core RunningLocalService
  -> 前端按 apiBaseUrl 通过 HTTP 使用 core
  -> 前端挂载 nativeBack handler registry 并完成事件订阅
  -> 前端 frontendReady -> 允许原生返回协商并隐藏 SplashScreen
```

## 边界

- 受信任 origin `https://winestock.internal` 仅由 `WebViewAssetLoader` 从本地 assets 提供，不经网络。
- Shell Bridge 消息通道和文档起始脚本都限定该 origin，非受信任 origin 无法调用桥。
- 业务能力通过 HTTP 使用 core；桥只承载运行配置、服务生命周期、真实地址和具名平台事件。
- 本地 self-hosted Axum 已接入；`server-mode` 前台服务仍未实现并通过 capability 关闭。
- 当前只构建/验收 ARM64 APK；API 33 三键导航 ARM64 真机已完成加载、HTTP、旋转、后台与
  force-stop 恢复 smoke，API 版本/手势导航/完整业务矩阵仍待覆盖。
