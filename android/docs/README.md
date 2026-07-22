# Android Shell 文档

本目录是 WineStock Android 平台 shell 的文档入口。
Android shell 负责 Activity 生命周期、WebView、前端资源打包加载和 Shell Bridge 传输，不拥有 core 业务规则、前端页面或运行设置 UI。

## Shell Bridge 实现

Android 实现 [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md) 定义的 Shell Bridge v1 契约，作为前端 `window.__WINESTOCK_SHELL_BRIDGE__` 的一个注入传输。
契约字段、错误码、快照结构和边界以该文档为权威，本目录不重复定义。

传输实现要点：

- 通过 `androidx.webkit` 的 `WebMessageListener` + document-start 脚本，在受信任 origin `https://winestock.internal` 上建立 JS ↔ Kotlin 消息通道。
- 前端资源由 `WebViewAssetLoader` 从 `assets/frontend` 提供；Gradle 使用本机 pnpm 构建当前源码，校验后注册为 variant generated assets。
- 信封协议、方法路由和快照派生写在代码注释中（`shell/bridge.js`、`shell/ShellBridgeHost.kt`）。

## WebView 原生返回协商

- `capabilities.nativeBack` 只在 AndroidX 消息通道、document-start shim 和 broker 成功安装后为 `true`。
- `MainActivity` 继续只通过生命周期感知的 `OnBackPressedDispatcher` 接收返回提交，不另建平台回调链。
- `ShellBridgeHost` 仅向当前可信、已调用 `frontendReady` 且 Activity 处于 resumed 的页面发送
  `nativeBackRequested { requestId, canGoBack }`；前端以 `resolveNativeBack` 一次性结算。
- `NativeBackRequestBroker` 同时最多保存一个 pending；等待期间重复返回直接消费，400ms 超时、
  `handled=false` 或发送失败时由 Activity 重新读取 `WebView.canGoBack()` 后 fallback。
- 主页面开始加载会推进 requestId 页面代次并清除旧 proxy；页面刷新、Activity pause/stop/destroy
  会取消 pending 且不额外 fallback，迟到或重复应答返回 `accepted=false`。
- 前端必须先完成 handler registry 与事件订阅，再调用 `frontendReady`。当前普通 Web fallback 保持
  `nativeBack=false`，不模拟 Android 系统行为。

## 当前边界与未实现

- 端上本地 Axum 尚未实现：本地模式返回 `unsupported_runtime_mode`，`startLocalService` 等能力为 `false`；远端模式为当前主用法。
- 运行配置校验是 shared 规则的 Kotlin 镜像（对齐前端 web fallback）；端上原生 Rust 服务落地后应改为委托 `winestock_shared`。
- 为连接局域网明文 HTTP 服务器，已放行 cleartext 与 WebView mixed-content，范围见 `network_security_config.xml` 与代码注释。
- 真实设备的手势导航、三键导航、旋转和后台恢复矩阵需要在有在线 Android 设备时统一执行；
  JVM、lint 与 assemble 不能替代该 smoke。

## 前端资源打包

- Android 构建直接从当前 `PATH` 执行本机 `pnpm run build:android`，不固定、下载或切换 Node/pnpm 版本。
- 前端依赖需要预先通过 `pnpm --dir frontend install --frozen-lockfile` 显式准备；普通 Android 构建不联网安装依赖。
- Vite Android mode 把产物写入 `app/build/intermediates/winestockFrontend/android/dist`，不读取 `frontend/dist`。
- `verify<Variant>FrontendAssets` 校验入口、manifest、资源引用、开发服务器标记和路径泄漏。
- `stage<Variant>FrontendAssets` 把通过校验的产物同步到 `app/build/generated/winestockFrontendAssets/<variant>/frontend`，并通过 AGP variant API 注册。
- `verify<Variant>FrontendPackage` 与 `verify<Variant>FrontendBundlePackage` 分别校验 APK/AAB 内的最终资源。
- `app/src/main/assets/frontend` 已废弃并受构建守卫禁止；`assets/shell/bridge.js` 仍是 Android 平台源码资源。

## WebView edge-to-edge 与安全区

Android shell 保持 `enableEdgeToEdge()`，让 Activity 根布局和 WebView 覆盖完整可绘制窗口；
`MainActivity` 不再给根容器统一添加 `systemBars` padding。统一避让由前端按内容语义完成，
避免状态栏/导航栏区域使用与前端不同的原生背景。

`web/WebViewportInsetsPublisher.kt` 负责：

- 读取 `systemBars | displayCutout`，不消费 WindowInsets；
- 依据当前 display density 把 Android 物理像素转换为 CSS 像素；
- 只向受信任 origin `https://winestock.internal` 发布
  `--shell-safe-area-inset-top/right/bottom/left`；
- 对相同数值去重，并在页面提交可见、加载完成、恢复或 inset 变化后重发；
- Activity 销毁时解除监听，不把 inset 扩展为 Shell Bridge v1 业务契约。

系统栏图标固定使用与当前浅色前端匹配的深色图标。夜间系统资源仍使用浅色
`web_background`，避免 SplashScreen、Window 和 WebView 空白期出现深色断层。

## 相关文档

- [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md)：Shell Bridge v1 契约与边界（权威）。
- [`../../docs/code-map/android.md`](../../docs/code-map/android.md)：Android shell 源码结构。
- [`../../docs/platforms.md`](../../docs/platforms.md)：Android 平台职责。
