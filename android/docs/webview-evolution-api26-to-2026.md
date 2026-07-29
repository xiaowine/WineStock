# Android WebView 演进与 WineStock 影响评估（API 26 至 2026-07-28）

> 调研日期：2026-07-28<br>
> 项目基线：`minSdk 28`、`targetSdk 36`、`compileSdk 37`、AndroidX WebKit `1.16.0`<br>
> 统计范围：Android 宿主 API、AndroidX WebKit、会改变嵌入式页面行为的 Chromium/WebView 里程碑；不逐条枚举 90 多个 Chromium
> 版本中的全部 Web Platform、V8、网络和安全修复。

## 结论

Android API level、AndroidX WebKit 版本和设备上的 Chromium WebView 版本是三个独立维度：

- API 26 设备并不等于只能运行 2017 年的 WebView；Android System WebView/Chrome 可以独立更新。
- 同一 APK 在 API 33、API 36 上可能因为 WebView 为 M116 或 M149 而表现不同。
- `compileSdk` 只决定编译时能使用的 API；`targetSdk` 才会开启一部分平台行为变更；设备 WebView 更新又能在不更新 APK
  的情况下改变网页视口和渲染行为。

按本报告口径，API 26 发布以来共有以下可量化变化面：

| 统计项                             |          数量/范围 | 说明                                                                                                  |
|---------------------------------|---------------:|-----------------------------------------------------------------------------------------------------|
| Android 平台                      | 12 个 API level | API 26（Android 8.0）至 API 37（Android 17 Beta）；项目当前 target 到 API 36                                   |
| `android.webkit` 公共 API 发生变化的平台 |            8 个 | API 26、27、28、29、30、33、35、37；31、32、34、36 没有新的 `android.webkit` 包级公共 API 差异页                          |
| AndroidX WebKit 稳定版本线           |           17 条 | `1.0.x` 至 `1.16.x`；`1.17.0` 截至调研日仍是 alpha                                                           |
| Chromium 主版本                    |   约 91 个已发布里程碑 | Android 8.0 同期约 M60，到官方已发布说明的 M150；Chromium Dashboard 已出现 M151 Android stable 条目，处于 M150/M151 滚动切换期 |
| 本项目已知真机 WebView                 |        2 个代表版本 | `116.0.5845.92`（早于 Insets 三个关键里程碑）和 `149.0.7827.163`（晚于 M144）                                       |

对 WineStock 的判断：

1. **当前 M136/M139/M144 的 Insets/IME 处理是正确的。** Native 统一拥有 Insets，向 WebView 下发把
   `systemBars | displayCutout | ime` 置零的副本，同时不返回 `WindowInsetsCompat.CONSUMED`，与 Android 官方的防 double
   padding / ghost padding 指引一致。
2. **最低 WebView 内核版本已落实为 M111。** Android 前端显式使用 `build.target: "chrome111"`；Activity 在创建任何
   WebView 前同时检查实际 provider 主版本与 `WEB_MESSAGE_LISTENER`、`DOCUMENT_START_SCRIPT` 两项必需能力。未知、过旧或能力
   缺失时进入原生恢复页，避免页面在 Vue 挂载前解析失败而留下空白屏。
3. **renderer 退出已有可控恢复链。** `ShellWebViewConfigurator` 处理 `onRenderProcessGone()`，记录 crash/系统回收、
   退出优先级和 provider 版本；`MainShellCoordinator` 销毁失效 WebView 后重建 Bridge、Insets 与页面。该路径不重启
   Application 级本地 core，仍需补充真实设备的 renderer kill/crash 注入验收。
4. **`targetSdk 37` 是下一次明确的迁移点。** Android 17 将 `ACCESS_LOCAL_NETWORK` 变成面向 API 37 应用的强制运行时权限，WebView
   的局域网请求继承宿主权限；WineStock 的“连接远端局域网 HTTP 服务”会直接受影响。当前 `targetSdk 36` 尚不会强制触发。

## 三条版本轴

### Android 系统

系统版本决定 Framework API、系统栏、输入法、返回分发、网络权限和 target SDK 行为。它不固定设备实际运行的 Chromium 版本。

Android 7 至 9 的 WebView 与 Chrome 共用安装包/大量代码，但不共享浏览数据；Android 10 起再次表现为两个独立应用。无论哪种形态，WebView
都是可更新组件，因此排障时必须同时记录 `SDK_INT`、`targetSdk` 和 `WebView.getCurrentWebViewPackage()`。

### Chromium WebView

Chromium 版本决定 HTML/CSS/JavaScript、网络栈、Visual Viewport、`safe-area-inset-*`、摄像头媒体能力和大量安全修复。它可以通过系统组件更新独立改变已有
APK 的表现。

### AndroidX WebKit

AndroidX WebKit 是兼容层，不替换 Chromium 内核。它让应用通过 `WebViewFeature.isFeatureSupported()` 在旧 Android 系统上调用新
WebView 能力。只检查 Android API level 不足以判断某项 WebView 功能是否可用。

WineStock 已正确对 `WEB_MESSAGE_LISTENER`、`DOCUMENT_START_SCRIPT` 和 `ALGORITHMIC_DARKENING`
做运行期能力探测；但前两个能力同时缺失时，Android Shell Bridge 会不可用，因此仍需一个明确的最低内核策略。

## Android API 26 至 37 时间线

| API | 系统版本            | WebView/宿主关键变化                                                                                                             | 对 WineStock 的影响                                                                       |
|----:|-----------------|----------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------|
|  26 | Android 8.0     | 增加 renderer 优先级、`getCurrentWebViewPackage()`、`onRenderProcessGone()`、Safe Browsing 开关；多进程 renderer 成为可管理边界                 | **历史高影响节点**：前台 renderer 请求 IMPORTANT 优先级且不可见时可降级；退出后重建 WebView，并记录 provider 版本。API level 本身不代表前端兼容                 |
|  27 | Android 8.1     | 增加 Safe Browsing 初始化、allowlist、命中回调和威胁类型                                                                                   | 低：主页面是受信任 asset origin，业务 API 不作为页面导航；若允许外部页面进入 WebView 则需要处理                         |
|  28 | Android 9       | 增加 tracing、`disableWebView()`、多进程 `setDataDirectorySuffix()`；target 28 后明文网络默认禁用；`file:` MIME 嗅探和根滚动元素更标准化                 | 中：当前单进程无需 suffix；项目已显式放开 cleartext。使用 HTTPS asset origin 而非 `file:`，规避本地文件安全与 MIME 问题 |
|  29 | Android 10      | 增加 `WebViewRenderProcessClient` 卡顿/恢复回调和 Force Dark；WebView/Chrome 再次作为独立应用呈现                                              | 中：没有 renderer 无响应监测；旧 Force Dark 已由算法着色方案取代                                           |
|  30 | Android 11      | AppCache 和危险 file-URL 访问 API 进入废弃路径；API 30 起若不显式设置，file/content 相关访问默认更收紧                                                  | 低：项目不使用 AppCache、`file:` 页面或 file-URL 跨域能力，采用 `WebViewAssetLoader`                    |
|  31 | Android 12      | target 31 后启用现代 SameSite：无属性按 `Lax`，`None` 必须配合 `Secure`，HTTP/HTTPS 视为跨站                                                   | 低：当前业务请求统一 `credentials: "omit"`，refresh token 存在 localStorage，不依赖 Cookie             |
|  32 | Android 12L     | `android.webkit` 无包级公共 API 增量；大屏、分屏和可调整窗口更加重要                                                                              | 中：WebView-Window 相交与横屏/分屏仍需矩阵测试，当前只覆盖部分横竖屏                                            |
|  33 | Android 13      | 移除 AppCache API；算法着色替代 Force Dark；引入 predictive back 基础机制                                                                  | 已处理：项目按能力关闭算法着色，使用自己的双主题；返回入口使用 `OnBackPressedDispatcher`                             |
|  34 | Android 14      | `android.webkit` 无包级公共 API 增量；predictive back 继续推进                                                                         | 已处理：返回入口使用 `OnBackPressedDispatcher`；产品只要求提交结果正确，不要求 Web 内容跟随手势进度动画                    |
|  35 | Android 15      | target 35 后强制 edge-to-edge；状态栏/手势栏透明，三键栏默认 80% 不透明保护；`WebSettings` 的 WebSQL database API 废弃                                | **已处理的高影响项**：项目主动 edge-to-edge、自己消费安全区，并用 `ProtectionLayout` 处理三键栏保护；不使用 WebSQL       |
|  36 | Android 16      | target 36 时 edge-to-edge opt-out 在 Android 16 上失效；predictive back 默认启用，旧 `onBackPressed`/`KEYCODE_BACK` 分发不再可靠；局域网权限可选择性测试 | 已处理：Activity 1.13 + `OnBackPressedDispatcher`，未使用旧返回入口；IME 可见时第一次返回只隐藏键盘。仅剩 API 36 真机返回矩阵验收 |
|  37 | Android 17 Beta | 新增文件夹选择/读写 permission mode；`startSafeBrowsing()` 废弃；支持时可用 ECH；target 37 后局域网访问强制 `ACCESS_LOCAL_NETWORK`                    | **下一次升级阻塞项**：远端 LAN 模式需声明、请求和解释权限；需验证 loopback、自托管和 LAN 的权限边界。当前图片文件选择不需要文件夹/写权限      |

`compileSdk 37` 不会提前开启 API 37 行为。只有将 `targetSdk` 改为 37 并运行在相应系统上，才会触发 target-gated
的强制局域网权限等变化。

## AndroidX WebKit 演进

| 稳定版本       | 时间        | 主要能力                                                                       | 本项目状态                                                              |
|------------|-----------|----------------------------------------------------------------------------|--------------------------------------------------------------------|
| 1.1        | 2019      | `WebViewAssetLoader`、Proxy、Tracing、renderer 管理兼容 API                       | 使用 `WebViewAssetLoader`；renderer 恢复使用 API 26 平台回调                            |
| 1.2-1.4    | 2020      | Force Dark、受 origin 约束的 `WebMessageListener`、多进程探测、Safe Browsing allowlist | 使用受信任 origin 消息通道；旧深色 API 不再使用                                     |
| 1.5        | 2022      | `setAlgorithmicDarkeningAllowed()`，面向 target 33 替代 Force Dark              | 已按能力显式设为 `false`                                                   |
| 1.6-1.8    | 2023      | 进程级启动前配置、Cookie 详情、X-Requested-With 策略、ArrayBuffer 消息                      | 当前无直接业务依赖                                                          |
| 1.9        | 2023      | 多 Profile、`addDocumentStartJavaScript()`、UA Client Hints 覆盖                | Shell Bridge 依赖 document-start + origin allowlist，且在 `loadUrl` 前安装 |
| 1.10-1.12  | 2024      | Media Integrity 控制、音频静音、RFC 6266 文件名、Back/Forward Cache、预渲染、WebAuthn       | 当前未启用，不构成升级要求                                                      |
| 1.13       | 2025-03   | 全量浏览数据删除、分区 Cookie、M133 网络流量标记、异步启动、prefetch；废弃主动 `startSafeBrowsing()`    | 当前不依赖 Cookie/预取；可考虑未来做 renderer/启动诊断                               |
| 1.14       | 2025-06   | Payment Request、实验 Navigation API                                          | 未启用，符合当前业务范围                                                       |
| 1.15       | 2025-12   | minSdk 升到 23；请求拦截 Cookie、静态自定义头、prerender、BFC 控制                           | 项目 `minSdk 28` 无升级障碍                                               |
| 1.16       | 2026-05   | minSdk 升到 24；异步启动和 Navigation Listener 稳定化；可控 `saveState()`                | **项目当前版本，也是截至调研日最新稳定线**                                            |
| 1.17 alpha | 2026-06 起 | HTTP cache quota、favicon 控制等预览能力；并新增未处理 renderer 回调的 lint                  | 不应仅为“追新”切到 alpha；renderer 回调已处理，仍不引入 alpha                       |

## Chromium/WebView 关键里程碑

这里不列出每个 Chromium 版本的全部网页标准变化，只列与本项目现有实现直接相交的节点。

| 里程碑       | 变化                                                                         | WineStock 关联                                                                                     |
|-----------|----------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| M60（2017） | Android 8.0 发布同期基线                                                         | 历史上说明仅靠 API level 可能面对非常旧的 JS/CSS 引擎；当前项目已用 M111 门禁关闭该不确定性                                      |
| M80/M89   | 现代 SameSite 逐步进入 Chrome/WebView；Android 12 文档要求测试至少 M89                    | 当前不使用 Cookie 鉴权，风险低                                                                              |
| M100      | User-Agent 主版本进入三位数                                                        | 项目不按 UA 猜测 Shell 能力，使用 capability，方向正确                                                           |
| M108      | 动态视口单位 `dvh/svh/lvh` 成为可用能力                                                | 前端大量使用 `100dvh`；低于该版本时布局不受当前构建承诺保护                                                               |
| M111      | 当前 Android 产物显式声明的 Chrome 语法目标下限                                          | **最低支持 WebView**；Native 启动门禁还要求 Shell Bridge 两项 capability                                                     |
| M116      | 一台测试机的旧代表版本                                                                | 早于 M136/M139/M144；没有新版 WebView 原生 Insets 行为，但高于当前 Vite 下限                                        |
| M136      | 仅全屏 WebView 将 `displayCutout()`/`systemBars()` 映射到 CSS `safe-area-inset-*` | Shell 自己发布安全区时可能重复；当前已向 WebView 置零规避                                                             |
| M139      | 所有 WebView 的 IME 开始直接缩小 visual viewport（仅底边交叠）                             | Native 再给根布局/页面加 IME padding 会双重避让；当前由内部内容容器消费并向 WebView 置零                                      |
| M144      | 所有 WebView 都把 `displayCutout()`/`systemBars()` 映射到 CSS safe area，不再仅限全屏    | M149 三键导航问题的直接背景；当前修复符合官方建议                                                                      |
| M149      | 一台测试机的新版代表版本                                                               | 已覆盖 M144 后行为；三键导航、IME 与原生返回已经人工验证                                                                |
| M150/M151 | 调研日的上游滚动边界                                                                 | 官方 Chrome release notes 已发布到 M150；Dashboard 已出现 M151 Android stable 条目，应继续把“当前版 + 前一版”纳入发布 smoke |

### M136、M139、M144 的准确含义

这三个编号是 Chromium/WebView 的主版本里程碑，不是 Android API level：

- M136：全屏 WebView 才接收状态栏/导航栏/挖孔 Insets，并映射为 CSS `safe-area-inset-*`。
- M139：所有 WebView 的 IME 底部交叠会缩小 visual viewport。
- M144：状态栏/导航栏/挖孔 CSS safe area 扩展到所有 WebView。

WebView 只应在系统 UI 与自身屏幕矩形相交时收到非零 Insets。Native 如果已经把某类 Insets 用作 padding，应通过新的
`WindowInsetsCompat.Builder` 把该类型改为 `Insets.NONE` 再继续向子树分发。直接返回 `CONSUMED` 会阻断后续零值通知，键盘收起后可能留下
ghost padding。

当前 `WebViewportInsetsPublisher` 正是这个所有权模型：

```text
Activity 根节点读取原始 Insets
  -> system bars / cutout 换算为 Shell CSS 变量
  -> IME 只压缩内部 WebView 内容容器
  -> 向 WebView 继续分发“已处理类型为 0”的 Insets 副本
  -> WebView M116 与 M149 都只看到一个稳定的内容契约
```

## 项目影响清单

### P0：发布前应明确

#### 1. 最低 WebView 版本（已落实）

Android Vite 构建显式设置 `build.target: "chrome111"`；转换只处理语法，不自动补齐 Web API polyfill。前端还使用了
`100dvh`、`visualViewport`、WebAssembly、MediaDevices 和动态模块，因此 Native 同时执行运行时门禁。

产品支持线为 **Android API 28+ 且 WebView M111+**。启动门禁记录：

- provider package；
- `versionName`/主版本；
- Shell Bridge 两个关键 `WebViewFeature`；
- 不支持时显示原生 WebView 版本提示页，只提供“重新检测”；用户在系统侧完成更新或启用后返回应用手动复检，不由应用跳转商店或系统设置，也不创建 WebView 或显示空白页。

如果产品必须支持低于 M111 的 WebView，需要显式降低 Vite target、审计全部 CSS/Web API 并引入必要
polyfill；仅修改一行构建目标并不能构成支持承诺。

#### 2. renderer 退出恢复

当前自定义 `WebViewClient` 已覆盖 `onRenderProcessGone()`。renderer 被系统回收或 crash 时，协调器记录
`didCrash`、`rendererPriorityAtExit` 与当前 provider package/version，取消旧页面的文件/摄像头请求，销毁旧 WebView，
并重新安装 Bridge、Insets、权限和既有返回链后加载受信任前端入口；不复用失效 WebView，也不重启 Application 级 core。
当前仍需完成以下真机验收：

- renderer 被系统回收与 renderer crash 的区分；
- 避免重复启动 Application 级本地 core；
- 保留可诊断日志但不泄漏用户数据。

#### 3. 为 target 37 预留 LAN 权限迁移

当前远端模式允许 WebView 从 `https://winestock.internal` 请求局域网 HTTP 地址。target 37 后应在应用层明确：

- 何时声明并请求 `ACCESS_LOCAL_NETWORK`；
- 用户拒绝后如何保持设置页可用并显示稳定错误；
- `127.0.0.1` 自托管是否被系统明确豁免，并用 API 37 真机验证；
- 远端 HTTPS/LAN HTTP、健康检查和业务请求是否得到一致权限结果；
- 权限不能被塞进 Shell Bridge 的业务 DTO，仍由 Android shell 拥有。

### P1：建议补强

#### 外部导航边界

当前 `WebViewClient` 负责 asset 拦截，但没有覆盖 `shouldOverrideUrlLoading()`。Bridge 已在非受信任页面上失效，这能保护
Native 能力，却不能阻止外部页面留在应用 WebView 内。建议只允许 `https://winestock.internal` 主文档导航，HTTP/HTTPS 外链统一交给现有
`openExternal()` 安全路径。

#### 明文与 mixed content

项目为局域网 HTTP 显式设置了 `usesCleartextTraffic="true"`、Network Security Config 和 `MIXED_CONTENT_ALWAYS_ALLOW`
。这是有意兼容当前部署方式，但作用面较宽。未来 server-mode TLS/pinning 完成后，应缩小明文域范围；在此之前把 mixed-content
warning 视为已知风险，而不是无关噪声。

#### 返回手势产品边界

产品只要求 Android 16 上的返回提交语义可靠，不要求 Web 内容跟随 predictive back 手势进度。当前
`OnBackPressedDispatcher` 路径满足 target 36 要求，未依赖旧 `Activity.onBackPressed()` 或 `KEYCODE_BACK`；IME 可见时
第一次返回只隐藏并消费键盘。gesture started/progressed/cancelled、Web 跟手动画和目的页面预览均为明确非目标，不作为缺口或后续待办。

### 已经规避或影响较低

| 变化                      | 当前结论                                                                                                                    |
|-------------------------|-------------------------------------------------------------------------------------------------------------------------|
| 现代 SameSite / 分区 Cookie | 业务请求 `credentials: "omit"`，token 不走 Cookie，影响低                                                                          |
| AppCache 移除             | 未使用                                                                                                                     |
| WebSQL database 废弃      | 未使用                                                                                                                     |
| file-URL 安全收紧           | 使用受信任 HTTPS asset origin；未启用 file-URL 跨域                                                                                |
| 算法深色                    | 已按能力显式关闭，页面使用自有主题                                                                                                       |
| 文件选择                    | 已由 SAF/Activity Result 返回 `content://`，不申请共享存储权限；API 37 文件夹/写模式不属于当前图片上传需求                                              |
| 摄像头                     | HTTPS asset origin、Native `CAMERA` 权限和受信任 origin 的 `onPermissionRequest()` 已具备；页面也做 `isSecureContext`/`getUserMedia` 检测 |
| WebView 多进程数据目录         | 应用当前只有主进程使用 WebView，不需要 suffix；未来新增进程时必须在首次加载 WebView 前配置                                                               |
| 安全区重复消费                 | 已按 M144 官方规则置零且继续分发，不使用 `CONSUMED`                                                                                      |

## 建议测试矩阵

不能把 API level 和 WebView milestone 合并成一个维度。最小有效矩阵如下：

| 维度          | 建议代表点                                                 | 目的                                                            |
|-------------|-------------------------------------------------------|---------------------------------------------------------------|
| Android API | 28、30、31、33、35、36、37 Beta                             | 覆盖 cleartext/file、SameSite、返回、edge-to-edge、未来 LAN 权限          |
| WebView     | 低于 M111（拒绝路径）、M111、M116、M136、M139、M144、M149/当前 stable | 覆盖构建下限和 Insets 三次行为切换                                         |
| 导航方式        | 手势、三键                                                 | 三键栏保护、返回与 IME 行为不同                                            |
| 窗口          | 竖屏、两个横屏方向、分屏/可调整窗口                                    | 验证 cutout、侧边导航和 WebView-Window 相交                             |
| 输入法         | 展开、返回关闭、切换输入法、三键/手势返回                                 | 验证第一下只关闭 IME、无双重 resize、无 ghost padding                       |
| renderer    | 系统回收、renderer crash/kill、页面 reload                    | 验证未来恢复链不停止本地 core、不遗留 Bridge pending                          |
| 网络          | loopback、LAN HTTP、远端 HTTPS、拒绝 LAN 权限                  | 验证明文策略和 API 37 权限边界                                           |

每次 Android 发布 smoke 至少记录：

```text
device model / SDK_INT / targetSdk
navigation mode / orientation / density
WebView provider package + full version
WEB_MESSAGE_LISTENER / DOCUMENT_START_SCRIPT feature result
layout viewport / visual viewport / native insets / CSS safe area
console error + Android crash/renderer termination log
```

## 后续优先级

1. 为 renderer kill/crash 增加真机恢复验收，确认本地 core 保持运行且无遗留 Bridge pending。
2. 增加主文档导航 allowlist，外链只走系统浏览器。
3. 保持现有 Insets 单一所有权，不再把 WebView 原生 `env()` 与 Shell 值相加。
4. target 37 前完成 `ACCESS_LOCAL_NETWORK` 交互、错误契约和真机矩阵。
5. 稳定性需求出现时再评估 AndroidX WebKit 1.16 的 Navigation/async startup；当前不切换到 1.17 alpha。

## 资料来源

以下资料均在 2026-07-28 核对；Android/AndroidX API 查询同时通过 Context7 的 Android Developers 官方文档库交叉确认。

- Android
  Developers：[Understand window insets in WebView](https://developer.android.com/develop/ui/views/layout/webapps/understand-window-insets)
- Android
  Developers：[Manage WebView objects](https://developer.android.com/develop/ui/views/layout/webapps/managing-webview)
- Android
  Developers：[Jetpack Webkit overview](https://developer.android.com/develop/ui/views/layout/webapps/jetpack-webkit-overview)
- AndroidX：[WebKit release notes](https://developer.android.com/jetpack/androidx/releases/webkit)
- Android 8-17 `android.webkit` JDiff：[`26`](https://developer.android.com/sdk/api_diff/26/changes/pkg_android.webkit)、[
  `27`](https://developer.android.com/sdk/api_diff/27/changes/pkg_android.webkit)、[
  `28`](https://developer.android.com/sdk/api_diff/28/changes/pkg_android.webkit)、[
  `29`](https://developer.android.com/sdk/api_diff/29/changes/pkg_android.webkit)、[
  `30`](https://developer.android.com/sdk/api_diff/30/changes/pkg_android.webkit)、[
  `33`](https://developer.android.com/sdk/api_diff/33/changes/pkg_android.webkit)、[
  `35`](https://developer.android.com/sdk/api_diff/35/changes/pkg_android.webkit)、[
  `37`](https://developer.android.com/sdk/api_diff/37/changes/pkg_android.webkit)
- Android
  9：[Behavior changes for apps targeting API 28](https://developer.android.com/about/versions/pie/android-9.0-changes-28)
- Android
  12：[Modern SameSite cookies in WebView](https://developer.android.com/about/versions/12/behavior-changes-12#samesite)
- Android
  15：[Behavior changes for apps targeting API 35](https://developer.android.com/about/versions/15/behavior-changes-15#edge-to-edge)
- Android
  16：[Behavior changes for apps targeting API 36](https://developer.android.com/about/versions/16/behavior-changes-16)
- Android 17
  Beta：[Behavior changes for apps targeting API 37](https://developer.android.com/about/versions/17/behavior-changes-17)
- Android
  Developers：[WebView unsafe file inclusion](https://developer.android.com/privacy-and-security/risks/webview-unsafe-file-inclusion)
- Chrome for Developers：[WebView overview](https://developer.chrome.com/docs/webview)
- Chrome for Developers：[Chrome 150 release notes](https://developer.chrome.com/release-notes/150)
- Chromium
  Dashboard：[Android Stable releases](https://chromiumdash.appspot.com/fetch_releases?channel=Stable&platform=Android&num=10)
- 本次问题来源文章：[记录 Android WebView 内核更新，安全区和 Insets 消费问题](https://jishuzhan.net/article/2031900985073926145)；其 M144 结论与当前 Android 官方 Insets 文档一致，项目判断以官方文档为准。

## 项目证据

- `android/app/build.gradle.kts`：API 28/36/37 基线。
- `android/gradle/libs.versions.toml`：AndroidX WebKit 1.16.0。
- `android/app/src/main/java/winestock/xiaowine/cc/web/WebViewportInsetsPublisher.kt`：Insets 单一所有权、IME
  容器避让和已处理类型置零。
- `android/app/src/main/java/winestock/xiaowine/cc/web/ShellWebViewConfigurator.kt`：asset、文件/摄像头、算法深色、mixed
  content、renderer 优先级与 renderer 退出回调配置；恢复编排由 `MainShellCoordinator` 完成。
- `android/app/src/main/java/winestock/xiaowine/cc/shell/ShellBridgeHost.kt`：消息通道、document-start 与
  origin/capability 约束。
- `android/app/src/main/java/winestock/xiaowine/cc/shell/NativeBackNavigator.kt`：IME 优先消费和前端/Activity 返回协商。
- `frontend/vite.config.ts`：Android 构建显式使用 Chrome 111 target。
- `android/app/src/main/java/winestock/xiaowine/cc/web/WebViewCompatibility.kt`：provider 主版本与必需 capability 启动门禁。
- `frontend/src/styles/foundation/_safe-area.scss`：Native/CSS 安全区取较大值而非相加。
- `frontend/src/api/client.ts`、`frontend/src/auth/storage.ts`：请求不携带 Cookie，refresh token 由 localStorage 管理。
