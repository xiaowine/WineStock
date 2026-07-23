# Android Shell 文档

本目录是 WineStock Android 平台 shell 的文档入口。
Android shell 负责 Activity 生命周期、WebView、前端资源打包加载和 Shell Bridge 传输，不拥有 core 业务规则、前端页面或运行设置 UI。

## Shell Bridge 实现

Android 实现 [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md) 定义的 Shell Bridge v1 契约，作为前端 `window.__WINESTOCK_SHELL_BRIDGE__` 的一个注入传输。
契约字段、错误码、快照结构和边界以该文档为权威，本目录不重复定义。

传输实现要点：

- 通过 `androidx.webkit` 的 `WebMessageListener` + document-start 脚本，在受信任 origin `https://winestock.internal` 上建立 JS ↔ Kotlin 消息通道。
- 前端资源由 `WebViewAssetLoader` 从 `assets/frontend` 提供；Gradle 使用本机 pnpm 构建当前源码，校验后注册为 variant generated assets。
- 运行配置与本地服务调用异步交给 Application 级 `LocalCoreRuntimeManager`；WebMessage/UI 线程不打开数据库或执行 JNI 启停。
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

## 本地 core 与当前边界

- `WineStockApplication` 在进程级持有一个 `LocalCoreRuntimeManager`；Activity 旋转或页面 reload 不停止本地服务。
- `android/native` 通过 JNI JSON protocol v1 调用 `winestock-core` 统一运行句柄，业务能力仍全部走 WebView HTTP。
- 权威配置校验来自 `winestock_shared`；Kotlin 只保留 native 无法加载时连接远端所需的最小降级校验。
- Android `self-hosted` 当前只允许 `127.0.0.1`；`server-mode` 在 Foreground Service 与通知策略完成前保持禁用。
- native library 无法加载时前端与设置页仍可打开，并允许保存/使用远端配置。
- 为连接局域网明文 HTTP 服务器，已放行 cleartext 与 WebView mixed-content，范围见 `network_security_config.xml` 与代码注释。
- API 33、三键导航的 ARM64 真机已完成 Debug APK 覆盖安装、JNI 实际加载、`self-hosted` migration、
  loopback `/api/health`、离线冷启动、远端/本机切换、旋转、后台恢复、force-stop 恢复和原生返回
  浮层/路由 smoke；未发现 404、Uncaught 或 FATAL，连接明文 HTTP 远端时出现的 WebView
  mixed-content warning 属于当前安全策略下的预期提示。
- 其它 API 版本、手势导航、异常注入和完整业务回归仍是剩余覆盖项；JVM、lint、assemble 和本次真机
  smoke 都不能替代完整矩阵。

## 前端资源打包

- Android 构建直接从当前 `PATH` 执行本机 `pnpm run build:android`，不固定、下载或切换 Node/pnpm 版本。
- 前端依赖需要预先通过 `pnpm --dir frontend install --frozen-lockfile` 显式准备；普通 Android 构建不联网安装依赖。
- Vite Android mode 把产物写入 `app/build/intermediates/winestockFrontend/android/dist`，不读取 `frontend/dist`。
- `verify<Variant>FrontendAssets` 校验入口、manifest、资源引用、开发服务器标记和路径泄漏。
- `stage<Variant>FrontendAssets` 把通过校验的产物同步到 `app/build/generated/winestockFrontendAssets/<variant>/frontend`，并通过 AGP variant API 注册。
- `verify<Variant>FrontendPackage` 校验 APK 内的最终前端资源；当前不注册 AAB 前端校验或 bundle 挂钩。
- `app/src/main/assets/frontend` 已废弃并受构建守卫禁止；`assets/shell/bridge.js` 仍是 Android 平台源码资源。

## Rust/ARM64 APK 打包

- 当前唯一 ABI 为 `arm64-v8a`，`minSdk/API` 为 26；不生成 32 位 ARM、x86 或 x86_64。
- `build<Variant>RustNativeLibraries` 使用预先准备的 `cargo-ndk 4.1.2` 和 NDK `30.0.14904198`，
  以 `--locked --offline` 构建；Debug 使用 Cargo debug profile，Release 使用 `--release`，因此
  `winestock-android-native -> winestock-core -> winestock-shared` 整条链都使用对应 profile。
- `utoipa-swagger-ui` 使用 vendored 资源，Debug 构建无需联网取得 Swagger UI；Release core 不注册 UI 路由，
  最终 `.so` 和 APK 不链接或打包 Swagger UI 静态资源，但继续提供 `/api-docs/openapi.json`。
- `.so` 只进入 `app/build/generated/winestockRustJniLibs/<variant>/arm64-v8a`，不写入源码树 `src/main/jniLibs`。
- `verify<Variant>RustNativeLibraries` 校验 ELF64/AArch64、JNI 导出、`DT_NEEDED` 和 profile marker；
  `verify<Variant>RustNativeApkPackage` 再检查最终 APK 只包含目标 ABI 和目标 `.so`。
- 当前交付物只支持 APK，不构建、不校验、不发布 AAB。

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

- [`release-package-size-analysis.md`](release-package-size-analysis.md)：Release APK/native library 实测组成、Swagger UI 移除结果与后续压缩方向。
- [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md)：Shell Bridge v1 契约与边界（权威）。
- [`../../docs/code-map/android.md`](../../docs/code-map/android.md)：Android shell 源码结构。
- [`../../docs/platforms.md`](../../docs/platforms.md)：Android 平台职责。
