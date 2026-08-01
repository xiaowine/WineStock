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

- IME 可见时，Activity 原生回调只请求隐藏输入法并消费本次返回；不发送前端返回请求，输入框焦点保留，
  后续再次返回才进入浮层、路由与 Activity 的常规优先级。
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

- `WineStockApplication` 在进程级持有一个 `LocalCoreRuntimeManager`；Activity 重建或页面 reload 不停止本地服务。
- `android/native` 通过 JNI JSON protocol v1 调用 `winestock-core` 统一运行句柄，业务能力仍全部走 WebView HTTP。
- 权威配置校验来自 `winestock_shared`；Kotlin 只保留 native 无法加载时连接远端所需的最小降级校验。
- 首次缺少 SharedPreferences 配置时只发布 `initialized=false` 和默认草稿，不创建配置、不启动本地
  Axum HTTP 服务；前端选择模式并成功 apply 后才启动本地 core 或切换远端。
- 已有有效配置的后续冷启动仍自动激活；首次确认 `self-hosted` 时以 `port=0` 请求系统分配端口，
  启动成功后 Shell 只持久化并发布实际非零端口；
  已保存端口冲突时自动换端口一次，后续进程启动优先复用最新端口。
- native `running` 状态中的配置端口、`boundAddress` 和 loopback `apiBaseUrl` 必须使用同一实际端口，
  不允许临时端口 `0` 进入运行快照或持久化配置。
- Android `self-hosted` 当前只允许 `127.0.0.1`；`server-mode` 在 Foreground Service 与通知策略完成前保持禁用。
- native library 无法加载时前端与设置页仍可打开，并允许保存/使用远端配置。
- 为连接局域网明文 HTTP 服务器，已放行 cleartext 与 WebView mixed-content，范围见 `network_security_config.xml` 与代码注释。
- API 33、三键导航的 ARM64 真机已完成 Debug APK 覆盖安装、JNI 实际加载、`self-hosted` migration、
  loopback `/api/health`、离线冷启动、远端/本机切换、原有旋转、后台恢复、force-stop 恢复和原生返回
  浮层/路由 smoke；未发现 404、Uncaught 或 FATAL，连接明文 HTTP 远端时出现的 WebView
  mixed-content warning 属于当前安全策略下的预期提示。
- `MainActivity` 通过 Manifest 的 `sensorPortrait` 锁定竖屏，允许正反竖屏传感器切换但禁止进入横屏；该规则
  尚需在代表设备上复验。其它 API 版本、手势导航、异常注入和完整业务回归仍是剩余覆盖项；JVM、lint、assemble
  和本次真机 smoke 都不能替代完整矩阵。首次未初始化不启服、选择模式后再 apply 的新漏斗尚未完成真机复验。

## 品牌与平台图标

- 跨平台权威母版位于仓库根 `brand/`；Android `res/drawable` 只保存 VectorDrawable 派生资源。
- launcher 使用直接颜色背景和冷白 Cube 前景；round icon 与 themed monochrome 复用同一个 adaptive/前景资源，前景按真机启动器的视觉安全区缩放，圆形与圆角方形遮罩保留稳定留白。
- SplashScreen 使用比界面标志更紧凑的专用 Cube 派生路径，避免系统启动图标容器把母版放大得过重；不兼容 WebView 恢复页继续使用正常尺寸的透明标志。两者通过 day/night 颜色资源保证浅色、深色背景对比，不再使用模板机器人、绿色网格或文字 `W`。

## 前端资源打包

- Android 构建直接从当前 `PATH` 执行本机 `pnpm run build:android`，不固定、下载或切换 Node/pnpm 版本。
- 前端依赖需要预先通过 `pnpm --dir frontend install --frozen-lockfile` 显式准备；普通 Android 构建不联网安装依赖。
- Vite Android mode 把产物写入 `app/build/intermediates/winestockFrontend/android/dist`，不读取 `frontend/dist`。
- `verify<Variant>FrontendAssets` 校验入口、manifest、资源引用、开发服务器标记和路径泄漏。
- `stage<Variant>FrontendAssets` 把通过校验的产物同步到 `app/build/generated/winestockFrontendAssets/<variant>/frontend`，并通过 AGP variant API 注册。
- `verify<Variant>FrontendPackage` 校验 APK 内的最终前端资源；当前不注册 AAB 前端校验或 bundle 挂钩。
- `app/src/main/assets/frontend` 已废弃并受构建守卫禁止；`assets/shell/bridge.js` 仍是 Android 平台源码资源。

## Rust/ARM64 APK 打包

- 当前唯一 ABI 为 `arm64-v8a`，`minSdk/API` 为 28；不生成 32 位 ARM、x86 或 x86_64。
- `build<Variant>RustNativeLibraries` 使用预先准备的 `cargo-ndk 4.1.2` 和 NDK `30.0.14904198`，
  以 `--locked --offline` 构建；Debug 使用 Cargo debug profile，Release 使用 `--release`，因此
  `winestock-android -> winestock-core -> winestock-shared` 整条链都使用对应 profile。
- Android 适配 crate 的 package/library 名称统一为 `winestock-android` / `winestock_android`，最终加载文件为
  `libwinestock_android.so`；`android/native` 仍是该 JNI 适配层的源码目录。
- 工作区 Release profile 启用 fat LTO，并显式关闭 Cargo strip；最终 Android `.so` 的符号处理仍由
  Android Gradle Plugin/NDK 打包流程负责。
- Debug 构建启用 `debug-swagger-ui` feature 并使用 vendored Swagger UI 资源；Release 不启用该 feature，
  不编译或链接 `utoipa-swagger-ui`，也不注册 `/api-docs/openapi.json` 或 `/swagger-ui`。
- Release 构建通过 `LIBSQLITE3_FLAGS` 取消 SQLite bundled FTS3/FTS5，以移除当前业务未使用的全文索引代码。
- Release APK 启用 R8 minify、optimize 与 resource shrink；`src/main/keepRules/rules.keep` 只保留 JNI
  入口等平台边界需要稳定二进制名称的类/方法。
- Release APK 通过 packaging resources exclude 移除 `kotlinx-coroutines` 调试探针资源
  `DebugProbesKt.bin`；不排除 Kotlin builtins 元数据。
- Release APK 输出为 `app/build/outputs/apk/release/WineStock-<versionName>-release.apk`；名称由 AGP variant 输出属性生成，`output-metadata.json` 同步记录该名称，Debug APK 仍沿用默认命名。
- `.so` 只进入 `app/build/generated/winestockRustJniLibs/<variant>/arm64-v8a`，不写入源码树 `src/main/jniLibs`。
- `verify<Variant>RustNativeLibraries` 校验 ELF64/AArch64、JNI 导出、`DT_NEEDED` 和 profile marker；
  `verify<Variant>RustNativeApkPackage` 再检查最终 APK 只包含目标 ABI 和目标 `.so`。
- 当前交付物只支持 APK，不构建、不校验、不发布 AAB。

## WebView 文件选择

- HTML `<input type="file">` 由 `MainActivity` 的 `WebChromeClient.onShowFileChooser` 承接。
- 使用 `FileChooserParams.createIntent()`（失败时回退 `ACTION_GET_CONTENT`）启动系统选择器 / SAF；
  经 Activity Result 把 `content://` URI（含多选 ClipData）交回 WebView `ValueCallback`。
- `web/WebViewFileChooserSession` 拥有单 pending 回调：新请求 supersede、取消、destroy 均以 `null`
  结算一次；与单个 Activity Result launcher 配套，supersede 后的唯一结果结算新回调（不丢弃）。
- 不申请 `READ_EXTERNAL_STORAGE`、`READ_MEDIA_*`、`CAMERA` 或 `MANAGE_EXTERNAL_STORAGE`；
  仅依赖系统对用户所选 URI 的临时读取授权。相机 Intent / `getUserMedia` 不在当前范围。
- 通用权限与 URI 策略提示见 [`webview-file-selection-permissions.md`](webview-file-selection-permissions.md)。

## WebView edge-to-edge 与安全区

Android shell 保持 `enableEdgeToEdge()`，让 Activity 根布局和 WebView 覆盖完整可绘制窗口；
`MainActivity` 不再给根容器统一添加 `systemBars` padding。统一避让由前端按内容语义完成，
避免状态栏/导航栏区域使用与前端不同的原生背景。

`web/WebViewportInsetsPublisher.kt` 负责：

- 读取 systemBars/displayCutout，底边再与 navigationBars(ignoringVisibility)、
  tappableElement、mandatorySystemGestures 取较大值；仍为 0 且无侧栏导航时回退
  系统 `navigation_bar_height`；
- 只在 Activity 根监听原始 inset；系统栏/挖孔发布为 shell CSS 变量，IME 由内部 WebView 内容容器的
  bottom padding 消费，`ProtectionLayout` 根节点保持无 padding，避免 `ColorProtection` 移到键盘上方；
- 向 WebView 下发把 `systemBars | displayCutout | ime` 设为 `Insets.NONE` 的副本，避免 WebView M139/M144
  再次调整 visual viewport 或生成重复 CSS safe area；不使用 `WindowInsetsCompat.CONSUMED`，确保键盘收起等
  零值更新仍能继续分发；
- 依据当前 display density 把物理像素转换为 CSS 像素；
- 只向受信任 origin `https://winestock.internal` 发布
  `--shell-safe-area-inset-top/right/bottom/left`；
- 对相同数值去重，并在页面提交可见、加载完成、恢复或 inset 变化后重发；
- Activity 销毁时解除监听，不把 inset 扩展为 Shell Bridge v1 业务契约。

前端主滚动区 `.app-content-pane` 保持全高 edge-to-edge（内容可画进导航栏后方），
触底时用真实节点 `.app-content-pane__end-inset`（含 `--safe-area-bottom`）撑开
scrollHeight，保证最后一项可完整露出；不在 `.app-shell` 上用 padding 裁掉栏下沉浸区域。

系统栏在前端接管前跟随系统 day/night mode，前端加载后按当前主题发布图标基线。图片全屏查看时前端经
`WineStockSystemChrome`（`SystemBarAppearanceController` / `SystemBarAppearanceBridge`）
临时改为浅色图标，关闭后恢复当前主题基线；不经 Shell Bridge 业务契约。Activity 根布局使用 AndroidX
`ProtectionLayout`，Android 10 以上在透明底部系统栏下绘制随前端主题切换的 `ColorProtection`；浅色和深色
均使用与前端 `--color-surface` 相同的基色和 70% alpha，保留内容透出且不依赖系统或厂商自动 contrast scrim 的不稳定色差。
夜间系统资源使用深色 `web_background` 和浅色系统栏图标，使 SplashScreen、Window 与 WebView 空白期先与系统主题一致；前端首帧再接管可能与系统相反的手动偏好。
应用主题的 `android:isLightTheme` 同样按 day/night 资源切换，使 WebView 的 `prefers-color-scheme` 与系统一致；WebView 算法着色关闭，避免覆盖前端自有的双主题 CSS。
`MainActivity` 在 Manifest 中锁定 `sensorPortrait`，禁止 Activity 进入横屏；同时只接管 `uiMode` 配置变化，并原地刷新 Window/WebView 背景、系统栏与安全区；随后向保留的页面发送 `winestock:system-theme-refresh`，覆盖部分 WebView 只更新 media query 结果却不派发 `change` 的行为。系统深浅色切换不会销毁 WebView、重载前端或丢失当前路由，手动主题也不会被系统配置覆盖。反向竖屏仍由传感器处理，其它配置变化继续使用 Android 默认重建行为。
主题联动已在 API 33 三键导航真机覆盖系统浅/深、手动浅/深、图片查看临时系统栏覆盖与关闭恢复，以及 Activity 后台恢复。系统 `uiMode` 双向切换期间 PID、ActivityRecord、WebView 调试 target、当前路由、Dialog 和未提交输入均保持，事件日志没有 Activity create/destroy；测试结束恢复设备原系统模式和应用“跟随系统”偏好。

`MainActivity` 在膨胀静态 WebView 布局前执行 M111 + Shell Bridge 必需 capability 启动门禁，并负责系统入口与 Activity Result 注册；不兼容时使用不依赖 WebView 的原生全屏提示页，只提供手动复检，不从应用跳转商店或系统设置。门禁通过后的组装与业务接线在
`shell/MainShellCoordinator`。兼容性探针/恢复页、WebView 配置、文件选择、Splash、系统栏与返回分别在
`web/WebViewCompatibility` / `web/WebViewCompatibilityScreen`、
`web/ShellWebViewConfigurator`、`web/WebViewFileChooserHost`、`web/SplashFrontendGate`、
`web/SystemBarAppearanceController`、`shell/NativeBackNavigator`。

## 相关文档

- [`webview-evolution-api26-to-2026.md`](webview-evolution-api26-to-2026.md)：API 26 至 2026-07-28 的 Android/WebView/AndroidX WebKit 演进统计、项目影响分级与测试矩阵。
- [`release-package-size-analysis.md`](release-package-size-analysis.md)：Release APK/native library 实测组成、Swagger UI 移除结果与后续压缩方向。
- [`webview-file-selection-permissions.md`](webview-file-selection-permissions.md)：通用的 WebView 文件选择、URI 授权、媒体库与相机权限提示；不作为具体功能契约。
- [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md)：Shell Bridge v1 契约与边界（权威）。
- [`../../docs/code-map/android.md`](../../docs/code-map/android.md)：Android shell 源码结构。
- [`../../docs/platforms.md`](../../docs/platforms.md)：Android 平台职责。
