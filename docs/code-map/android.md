# Android 代码地图

`android` 是 Android 原生 shell，拥有 Activity 生命周期、WebView、打包前端资源加载和 Shell Bridge 传输实现。
它通过 HTTP 使用 core，通过 Shell Bridge 交换运行配置和服务状态，不拥有业务 API、不复制 core 业务实现，也不把业务 UI 托管给 Axum。
当前实现范围是**打包前端 + Application 级本地 core + 远端降级**：默认可在进程内启动共享 Axum/core，权威配置校验来自 `android/native -> winestock_shared`；native library 不可用时仍能打开设置页并连接远端。
逐文件职责以各源码文件注释为准；本地图记录模块所有权、边界和跨层流程。

## 工程与构建（`android/*.gradle.kts`、`buildSrc/`）

- 单 `:app` 模块工程，命名空间 `winestock.xiaowine.cc`；固定 NDK 与唯一 ABI `arm64-v8a`，交付物只支持 APK（无 AAB 校验或 bundle 挂钩）。
- 前端打包任务链：从当前 `PATH` 直接执行本机 `pnpm run build:android`（不固定或下载 Node/pnpm，不读取 `frontend/dist`），经目录校验、generated assets 暂存、legacy 守卫和 APK 包级验证；依赖未准备或产物缺失立即失败。
- Rust JNI 构建任务链：`cargo-ndk --locked --offline` 按 variant 构建 ARM64 `.so`（Release 走 Cargo `--release`，core/shared 传递依赖同 profile），并校验 ELF64/AArch64、8 个具名 JNI 导出、允许的系统动态库和 APK 内唯一 `.so`。

## Activity 装配与 Application 级 core

- `MainActivity.kt` 是唯一 Activity 入口，只保留系统生命周期与 ActivityResult；`shell/MainShellCoordinator.kt` 组装 edge-to-edge、Splash、inset、WebView、Shell Bridge、返回协商与文件选择 Host，`loadUrl` 前安装 Bridge。
- `WineStockApplication.kt` 在进程创建时初始化唯一 `LocalCoreRuntimeManager`；Activity 重建不替换 core runtime，Activity 生命周期不触发 shutdown。
- `core/LocalCoreRuntimeManager.kt`：单线程后台 executor 串行执行 JNI、配置校验、启动/停止/重启和 SharedPreferences 提交。缺少持久配置时只发布 `initialized=false` 且不启动本地 HTTP 服务；首次确认 `self-hosted` 用端口 `0` 请求系统分配并把实际端口写回持久化，已保存端口冲突时仅 `self-hosted` 自动重试一次；候选配置先激活后提交，失败时恢复旧服务。
- `core/` 其余模块：安全幂等加载 `.so` 并封装 native protocol v1 调用（load/JNI 失败映射 `native_library_unavailable`，不在类初始化时崩溃）；编解码请求/响应并拒绝协议版本不匹配，校验 `running` 状态发布的是同一非零实际端口；存储固定使用 `noBackupFilesDir/winestock/data`。
- `android/native/`：Cargo `cdylib`，Android 唯一 Rust 适配层，依赖 `winestock-core`/`winestock-shared`。每个 JNI 入口只交换 JSON 并阻止 panic 越过 FFI；持有双 worker Tokio Runtime 和 `RunningLocalService`；配置校验复用 shared 权威规则并额外限制 Android self-hosted 仅 `127.0.0.1`、禁用 server-mode。

## WebView 环境（`app/.../web/`）

- 资源加载：`WebViewAssetLoader` 把受信任 origin 根路径映射到 `assets/frontend`，根路径回退 `index.html`，不做 SPA 回退（前端使用 hash 路由）；`ShellWebViewConfigurator.kt` 集中 WebView 配置。
- 摄像头授权：`WebViewCameraPermissionHost.kt` 处理 `onPermissionRequest`——仅受信任 origin 的 VIDEO_CAPTURE 且原生 CAMERA 运行时权限获准后才 grant，权限未授时经 Activity launcher 请求后结算，其余来源与资源一律 deny；manifest 声明 `CAMERA` 并以 `uses-feature required=false` 保持无摄像头设备可安装。
- 渲染环境：WindowInsets 按 density 换算成 CSS 像素后仅在受信任 origin 写入 `--shell-safe-area-inset-*`（不扩展 Shell Bridge 业务契约）；系统栏浅色/深色外观经独立 JS 接口控制，resume 时重放。
- 文件选择：单 pending 回调的纯状态机 Session（supersede/cancel/destroy 一次结算，JVM 单元测试覆盖竞态）加启动系统选择器的 Host；不声明存储/媒体权限。
- `SplashFrontendGate.kt`：SplashScreen keep-on-screen 与超时/前端 ready 门闩。

## Shell Bridge 传输（`app/src/main/assets/shell/bridge.js`、`app/.../shell/`）

- `bridge.js` 是注入 WebView 的传输 shim（平台传输层，不是前端源码）：构造 `window.__WINESTOCK_SHELL_BRIDGE__`，以 call/reply/event 信封把 `frontend/src/shell/contract.ts` 的 v1 逻辑接口映射到原生消息通道，同时注入 `window.__WINESTOCK_RUNTIME_CONFIG__`；消息通道缺失时暴露降级桥。
- `ShellBridgeHost.kt`：在受信任 origin 注册 `WebMessageListener` 与文档起始脚本（带能力检测），把配置与本地服务生命周期异步路由到 Application manager 并经主线程回复；管理页面代次与迟到结果丢弃（`AsyncBridgeReplyGate.kt`）；`openExternal` 只放行不含凭据的 http/https。只处理具名平台能力，不代理业务 HTTP、不传递业务 access/refresh token；`self-hosted` 快照会携带 core per-boot 的本机会话换取凭据（`localAuthExchangeToken`，仅经受信任 origin 通道下发，不落日志），由前端换取正常 token 实现本机免登录。
- 原生返回：`NativeBackNavigator.kt` 先经 Bridge 协商，未处理时走 WebView history 或 finish；`NativeBackRequestBroker.kt` 是纯状态机（单 pending、400ms 超时、重复/迟到应答拒绝），由 JVM 单元测试覆盖竞态。
- 运行配置：四字段 `EditableRuntimeConfig` 模型与版本化 SharedPreferences 持久化（三态读取，与前端 `web.ts` 一致，只保存运行配置不保存 token）；`RuntimeSnapshotFactory.kt` 构造 v1 快照并发布 Shell 权威 `initialized`，`serverMode` 固定 false；native 不可用时仅校验远端地址降级路径。
- `AppConfig.kt`：受信任 host `winestock.internal`（ICANN 保留、永不进入公网 DNS）、允许 origin、Splash 超时和原生返回应答超时等 shell 常量。

## 资源与配置

- `res/`：全屏 WebView 布局、Window/SplashScreen 背景（系统 night mode 下仍保持浅色）、放行明文流量的 network security config。
- `AndroidManifest.xml`：仅 `INTERNET` 权限；文件选择走系统选择器。
- `app/build/` 下的前端中间输出与 generated assets 由 Gradle 生成和清理。

## 启动流程

```text
MainActivity.onCreate
  -> WineStockApplication.LocalCoreRuntimeManager（进程级异步初始化；只自动激活持久配置）
  -> WebViewAssetLoader（域名 winestock.internal，/ -> assets/frontend）
  -> ShellBridgeHost.install（WebMessageListener + document-start shim，限受信任 origin）
  -> WebView.loadUrl(https://winestock.internal/)
  -> 前端 getRuntimeSnapshot
     -> initialized=false：显示运行设置，不启动 core
     -> initialized=true：使用 Shell 已激活的本地/远端状态
  -> 用户首次 applyRuntimeConfig -> manager -> JNI -> android/native -> core RunningLocalService
  -> 前端按 apiBaseUrl 通过 HTTP 使用 core
  -> 前端 frontendReady -> 允许原生返回协商并隐藏 SplashScreen
```

## 边界与验收状态

- 受信任 origin `https://winestock.internal` 仅由 `WebViewAssetLoader` 从本地 assets 提供，不经网络；Bridge 通道和起始脚本都限定该 origin。
- 业务能力通过 HTTP 使用 core；桥只承载运行配置、服务生命周期、真实地址和具名平台事件。
- `server-mode` 前台服务仍未实现并通过 capability 关闭。
- 当前只构建/验收 ARM64 APK；API 33 三键导航真机已完成加载、HTTP、旋转、后台与 force-stop 恢复 smoke；首次未初始化延迟启服仍待真机复验，API 版本/手势导航/完整业务矩阵仍待覆盖。
