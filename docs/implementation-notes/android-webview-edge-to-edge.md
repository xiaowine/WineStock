# WineStock Android WebView Edge-to-Edge 实施方案

> 文档状态：已实施；已完成 API 36 手势导航与 API 33 三键导航基础验收，待完整设备矩阵<br>
> 涉及组件：`android`、`frontend`、根项目文档<br>
> 编制日期：2026-07-23<br>
> 适用范围：Android API 26 及以上，当前 `targetSdk = 36`

## 1. 结论

WineStock Android 应继续启用 edge-to-edge，但不再由原生层对整个 WebView 容器统一添加系统栏 padding。

目标方案为：

1. Android Window 与 WebView 铺满整个屏幕，包括状态栏、导航栏和挖孔区域背后的可绘制区域。
2. Android shell 继续拥有 WindowInsets 采集、系统栏图标明暗和 WebView 生命周期。
3. Android 将真实的 `systemBars | displayCutout` inset 转换为 CSS 像素并发布给受信任的前端页面。
4. 前端背景允许延伸到屏幕边缘，只有标题、按钮、表单、固定操作区等需要交互或阅读的内容进行安全区避让。
5. 前端以统一 CSS 变量消费安全区；浏览器原生 `env(safe-area-inset-*)` 只作为补充来源，不作为 Android WebView 的唯一权威来源。

不得删除 `enableEdgeToEdge()`。当前应用目标 SDK 已达到 36，Android 15 起对目标 SDK 35 及以上应用强制 edge-to-edge，删除该调用不能可靠恢复旧布局，只会导致旧系统和新系统行为不一致。

## 2. 背景与问题定义

### 2.1 当前 Android 行为

当前 `MainActivity` 执行以下流程：

```text
enableEdgeToEdge()
  -> setContentView(binding.root)
  -> applySystemBarInsets()
  -> 给 binding.main 设置 systemBars 四边 padding
```

因此实际布局并非真正的 WebView edge-to-edge：

```text
Android Window（全屏）
├─ 状态栏区域：Android 原生 window/root 背景
├─ WebView：被 padding 收缩到 systemBars 安全矩形内
└─ 导航栏区域：Android 原生 window/root 背景或系统保护层
```

相关证据：

| 位置                                            | 当前行为                            | 影响                                     |
| ----------------------------------------------- | ----------------------------------- | ---------------------------------------- |
| `android/app/build.gradle.kts:14`               | `targetSdk = 36`                    | Android 15 及以上强制 edge-to-edge       |
| `MainActivity.kt:57`                            | 调用 `enableEdgeToEdge()`           | Window 允许内容绘制到系统栏后方          |
| `MainActivity.kt:88-93`                         | 根容器统一应用 `systemBars` padding | WebView 又被整体缩回安全区               |
| `values-night/colors.xml:3`                     | `web_background = #121212`          | 系统深色模式时系统栏区域使用深色原生背景 |
| `frontend/src/styles/foundation/_tokens.scss:3` | `color-scheme: light`               | 前端当前只有浅色主题                     |

### 2.2 直接后果

- 状态栏和导航栏显示的是 Android 原生背景，而不是当前前端页面背景。
- Android 系统深色模式下，原生背景为深色，前端仍为浅色，产生明显割裂。
- `enableEdgeToEdge()` 与根容器统一 padding 同时存在，语义冲突，维护者难以判断当前到底是否真正 edge-to-edge。
- 前端已经存在多处 `safe-area-inset-*` 处理，但当前 WebView 位于安全矩形内，这些规则不能形成完整、可验证的 Android 安全区契约。
- 三键导航下 Android 可能额外绘制半透明保护层，导航栏颜色不能通过静态原生背景保证与状态栏完全一致。

### 2.3 前端已有基础与缺口

已有基础：

- `frontend/index.html` 已声明 `viewport-fit=cover`。
- 移动顶栏、导航 Drawer、认证页面、Dialog、Notice、图片预览以及部分底部操作区已经使用 `env(safe-area-inset-*)`。
- 全局应用壳使用 `100dvh`，具备全视口布局基础。

主要缺口：

- 缺少由 Android WindowInsets 提供的稳定、安全区数据源。
- 各组件直接使用原始 `env()`，没有统一抽象，容易发生遗漏和重复计算。
- 基本没有统一处理左右安全区，横屏挖孔设备存在遮挡风险。
- 普通滚动内容区缺少统一的底部可滚动安全留白。
- `ServiceUnavailableScreen` 等全屏状态尚未完整处理四边安全区。
- Android 系统栏图标颜色跟随系统夜间模式，而不是跟随当前前端浅色视觉。

## 3. 目标与非目标

### 3.1 目标

- WebView 实际边界与 Activity 可绘制窗口一致，不再由系统栏 padding 缩小。
- 状态栏和手势导航栏的透明区域自然显示前端背景。
- 交互内容在状态栏、导航栏、挖孔和横屏侧边区域内不被遮挡。
- 同一套前端源码在普通浏览器、Android WebView 和未来 Desktop shell 中拥有稳定的安全区回退行为。
- Android 8 至 Android 16、手势导航和三键导航均具备明确、可测试的结果。
- 不扩大 Shell Bridge 的业务边界，不影响 HTTP API、鉴权或运行配置协议。

### 3.2 非目标

- 不实现沉浸式隐藏状态栏或导航栏。
- 不在本次引入前端深色主题。
- 不实现 Android 端本地 Axum 服务。
- 不修改业务 API、鉴权、数据库或运行模式。
- 不为临时回退长期保留双套布局实现或运行时 feature flag。
- 不保证三键导航系统保护层与网页背景像素级完全相同。

## 4. 设计原则与职责边界

### 4.1 Android shell 负责

- 调用并保持 `enableEdgeToEdge()`。
- 让 WebView 填满 Activity Window。
- 采集 `WindowInsetsCompat.Type.systemBars()` 与 `displayCutout()`。
- 把 Android 物理像素转换成前端 CSS 像素。
- 在 inset、旋转、导航模式或窗口尺寸变化时更新前端变量。
- 管理状态栏和导航栏前景图标的明暗。
- 在 SplashScreen、WebView 空白期和加载失败时提供与前端一致的中性背景。

### 4.2 前端负责

- 决定哪些背景可以延伸到屏幕边缘。
- 决定哪些可读或可操作内容需要避让安全区。
- 使用统一安全区变量处理顶栏、Drawer、Sheet、Dialog、固定按钮、通知和滚动尾部。
- 保证每个受影响页面在移动、横屏和动态 inset 下仍可使用。
- 不通过 User-Agent 或 Android 全局对象猜测平台能力。

### 4.3 Shell Bridge 边界

本次不把 viewport inset 加入 Shell Bridge v1 的逻辑业务契约。

原因：

- inset 是 WebView 渲染环境数据，不是运行配置、服务生命周期或业务能力。
- Android 可以通过受信任页面的 CSS 自定义属性发布数值，不需要增加异步 RPC。
- Web 和 Desktop 默认值保持为 `0px`，无需实现无意义的桥方法。
- 避免仅为布局数据升级整个 Shell Bridge 协议版本。

## 5. 目标架构

```text
Android Window（edge-to-edge）
├─ 透明状态栏 / 导航栏
├─ WindowInsets 监听
│  └─ systemBars | displayCutout
│     └─ px -> CSS px
│        └─ --shell-safe-area-inset-*
└─ WebView（match_parent，全屏）
   ├─ 前端背景：允许绘制到屏幕边缘
   ├─ --safe-area-*：统一安全区值
   └─ 交互内容：按语义局部避让
```

数据流：

```text
Android WindowInsets
  -> WebViewportInsetsPublisher
  -> document.documentElement CSS variables
  -> frontend foundation safe-area variables
  -> AppShell / Dialog / 页面固定操作区 / 全屏状态
```

## 6. Android 实施设计

### 6.1 保留 edge-to-edge，移除统一 padding

`MainActivity` 保留：

```kotlin
enableEdgeToEdge()
```

删除当前 `applySystemBarInsets()` 对 `binding.main` 的统一 `setPadding()` 行为。

布局文件中的 WebView 已通过四边约束铺满父容器，不需要额外修改尺寸规则。

### 6.2 新增 WebViewportInsetsPublisher

建议新增：

```text
android/app/src/main/java/winestock/xiaowine/cc/web/WebViewportInsetsPublisher.kt
```

职责：

- 安装 `OnApplyWindowInsetsListener`。
- 读取 `systemBars() or displayCutout()` 的组合 inset。
- 缓存最新四边值，避免相同值重复执行 JavaScript。
- 将物理像素除以 `displayMetrics.density`，转换为 CSS px。
- 只拼接已经格式化的有限数值，不接受页面或用户输入，避免脚本注入风险。
- 页面提交可见后重新发布缓存值，覆盖页面刷新或重建后的变量丢失。
- Activity 销毁时停止继续向失效 WebView 发布。

示意代码：

```kotlin
ViewCompat.setOnApplyWindowInsetsListener(binding.main) { _, insets ->
    val safeInsets = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
            WindowInsetsCompat.Type.displayCutout(),
    )
    viewportInsetsPublisher.update(safeInsets)
    insets
}
```

监听器返回原 `insets`，不对整棵 View 层级执行统一消费或 padding。

### 6.3 CSS 像素换算

Android `Insets` 使用物理像素，CSS 使用 CSS px。不得把 Android 原始像素直接写成 CSS `px`。

第一版换算：

```text
cssPx = androidPhysicalPx / resources.displayMetrics.density
```

要求：

- 使用固定英文小数点格式输出。
- 最多保留两位小数。
- 小于可感知阈值的值归零。
- 旋转或显示缩放变化后重新计算，不能长期缓存 density 换算结果。

### 6.4 发布变量

Android 发布以下只读环境值：

```css
--shell-safe-area-inset-top: 0px;
--shell-safe-area-inset-right: 0px;
--shell-safe-area-inset-bottom: 0px;
--shell-safe-area-inset-left: 0px;
```

建议通过数字限定的脚本更新根元素：

```javascript
document.documentElement.style.setProperty(
  "--shell-safe-area-inset-top",
  "24px",
);
```

发布时机：

1. 首次收到 WindowInsets 时缓存。
2. 页面提交可见或加载完成时发布缓存值。
3. 后续 inset 变化时立即更新。
4. 页面还未建立 `documentElement` 时保留缓存，下一次页面可见回调重试。

当前 SplashScreen 会保持到前端就绪，可遮蔽首次零值到真实值之间的短暂布局调整。

### 6.5 系统栏图标外观

前端当前明确为浅色主题，因此默认系统栏图标应跟随前端主题，而不是 Android 系统夜间模式：

```kotlin
isAppearanceLightStatusBars = true
isAppearanceLightNavigationBars = true
```

这里的 `true` 表示浅色背景使用深色图标。

第一版不扩展 Shell Bridge 来动态切换系统栏图标。对于全屏图片预览等深色临时层，先让深色内容和操作控件限制在安全绘制区域内，状态栏和导航栏区域继续保留浅色背景。若产品后续明确要求图片预览也覆盖系统栏，再单独设计窄范围的系统栏外观桥能力。

### 6.6 SplashScreen 与原生背景

由于前端当前只有浅色主题：

- `values-night/colors.xml` 不应继续把 `web_background` 设置为 `#121212`。
- 普通和 night 资源中的启动背景应与前端 `--color-page` 或移动顶栏的浅色背景协调。
- WebView 的预绘制背景、Window background 和 SplashScreen background 使用同一颜色来源。
- 不在本次引入完整 Android 深色主题适配。

## 7. 前端实施设计

### 7.1 建立统一安全区变量

在 foundation 层定义平台原始值与应用统一值：

```scss
:root {
  --shell-safe-area-inset-top: 0px;
  --shell-safe-area-inset-right: 0px;
  --shell-safe-area-inset-bottom: 0px;
  --shell-safe-area-inset-left: 0px;

  --safe-area-top: max(
    env(safe-area-inset-top, 0px),
    var(--shell-safe-area-inset-top)
  );
  --safe-area-right: max(
    env(safe-area-inset-right, 0px),
    var(--shell-safe-area-inset-right)
  );
  --safe-area-bottom: max(
    env(safe-area-inset-bottom, 0px),
    var(--shell-safe-area-inset-bottom)
  );
  --safe-area-left: max(
    env(safe-area-inset-left, 0px),
    var(--shell-safe-area-inset-left)
  );
}
```

必须使用 `max()`，不得把 `env()` 与 shell 值相加。部分 WebView 版本可能同时提供非零 `env()`，相加会造成双重避让。

### 7.2 不给 html/body/#app 统一加安全区 padding

全局统一 padding 会重建当前 Android 根容器 padding 的问题：背景无法延伸到系统栏后方，所有页面还会被无差别压缩。

统一规则：

- 背景层、遮罩层和滚动背景允许 full bleed。
- 文本、按钮、输入控件和固定操作区按组件语义避让。
- 普通流式内容不因系统栏改变整体横向栅格，只有靠近对应边缘时才消费 inset。

### 7.3 应用壳

移动顶栏：

- 高度继续由基础顶栏高度加 `--safe-area-top` 组成。
- 左右 padding 使用基础值与左右安全区组合。
- 顶栏背景必须覆盖状态栏区域，使状态栏背景自然与顶栏一致。

主内容滚动区：

- 增加底部安全留白或 `scroll-padding-bottom`。
- 确保最后一项能够滚动到导航栏上方，而不是永远停在导航栏后面。
- 不要求每个普通页面重复声明相同底部 inset。

导航 Drawer：

- 背景继续覆盖整个屏幕高度。
- Header、导航列表首尾和关闭按钮分别消费四边安全区。
- 横屏时必须处理左侧或右侧 display cutout。

### 7.4 Dialog、Sheet 和全屏浮层

- `.modal-layer` 遮罩允许延伸到系统栏后方。
- 移动底部 Sheet 的操作区使用 `--safe-area-bottom`。
- 全屏工作区限制可操作内容在 top/right/bottom/left 安全区域内。
- 关闭按钮、Popover 和 Teleport listbox 不得只考虑顶部 inset。
- 嵌套 Dialog 继续复用同一安全区变量，不单独计算平台类型。

### 7.5 全局状态

以下状态必须显式审计：

- 前端视口稳定前的 bootstrap 背景。
- 登录、注册和强制改密页面。
- 运行设置页面。
- 服务不可用覆盖层。
- 全局 Notice。
- 移动导航 Drawer。
- 普通与嵌套 Dialog。
- 全屏图片预览。
- 入库、出库和审批页面的固定底部操作区。

### 7.6 现有直接 env() 使用迁移

使用以下命令建立完整迁移清单：

```powershell
rg -n "safe-area-inset" frontend/src
```

所有业务组件应改用 `--safe-area-*`，不再直接组合原始 `env()`。这样 Android 原生值、浏览器值和未来平台值只有一个合并入口。

## 8. 文件级变更清单

| 组件     | 文件                                                                                                         | 已实施变更                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Android  | `MainActivity.kt`                                                                                            | 保留 edge-to-edge；移除根 padding；接入 inset publisher；固定浅色前端对应的系统栏图标；页面可见时重发 inset |
| Android  | `web/WebViewportInsetsPublisher.kt`                                                                          | 新增 WindowInsets 采集、CSS px 转换、去重缓存与受控变量发布                                                 |
| Android  | `res/values-night/colors.xml`                                                                                | 移除与浅色前端冲突的深色 `web_background`                                                                   |
| Android  | `res/values/colors.xml`、`themes.xml`                                                                        | 统一 Window、SplashScreen 与 WebView 加载前背景                                                             |
| Android  | `android/docs/README.md`                                                                                     | 记录 edge-to-edge 和 inset 发布职责                                                                         |
| Android  | `docs/code-map/android.md`                                                                                   | 增加新 publisher 模块和系统栏职责                                                                           |
| Frontend | `styles/foundation/_tokens.scss` 或新的 `_safe-area.scss`                                                    | 定义 shell 原始变量与统一 `--safe-area-*`                                                                   |
| Frontend | `styles/index.scss`                                                                                          | 新增 safe-area foundation 文件时负责装配                                                                    |
| Frontend | `layouts/AppShell.scss`                                                                                      | 顶栏、内容滚动区和 Drawer 使用统一安全区变量                                                                |
| Frontend | `_auth.scss`、`ModalDialog.scss`、`ServiceUnavailableScreen.scss`、`PreviewImage.scss`、`NoticeViewport.vue` | 覆盖认证、浮层、全屏状态和通知                                                                              |
| Frontend | 所有 `rg safe-area-inset` 命中的页面样式                                                                     | 迁移直接 `env()` 并补齐左右安全区                                                                           |
| Frontend | `frontend/docs/README.md` 与移动交互文档                                                                     | 建立安全区规范和验收入口                                                                                    |
| 根文档   | `docs/platforms.md`、相关代码地图                                                                            | 同步当前 Android shell 状态与 edge-to-edge 所有权                                                           |

## 9. 分阶段实施

### 阶段 0：基线记录

预计：0.5 人日。

- 在当前实现下记录手势导航、三键导航、系统浅色和深色模式截图。
- 记录 WebView、根容器和系统栏的实际 bounds。
- 确认至少一个挖孔或横屏模拟设备。
- 建立本方案受影响页面清单。

### 阶段 1：Android 全屏 WebView 与 inset 发布

预计：0.5～1 人日。

- 删除根容器统一 padding。
- 新增 `WebViewportInsetsPublisher`。
- 保持 edge-to-edge 并统一系统栏图标。
- 对齐 Window、SplashScreen 和 WebView 背景。
- 验证前端暂未消费新变量时只有内容遮挡，不出现崩溃或桥异常。

### 阶段 2：前端安全区基础设施与应用壳

预计：0.5～1 人日。

- 增加统一变量。
- 迁移 `AppShell`、认证页、运行设置和服务不可用页。
- 补齐内容滚动尾部和左右安全区。
- 保证普通浏览器默认值为零，不改变桌面布局。

### 阶段 3：业务浮层和固定操作区审计

预计：1～1.5 人日。

- 迁移所有直接 `safe-area-inset-*` 使用。
- 检查 Dialog、Sheet、图片预览、Notice、Drawer。
- 检查入库、出库、审批、库位等移动端固定操作区。
- 处理横屏 display cutout 与底部最后一项可达性。

### 阶段 4：设备验证、文档与收尾

预计：1 人日。

- 执行 Android 版本、导航模式和方向矩阵。
- 修复控制台错误、横向溢出和双重 inset。
- 更新 Android、frontend 与根代码地图。
- 清理旧注释、失效函数和原生 night 背景规则。

总计预计：3.5～5 人日，主要不确定性来自真实设备和三键导航/挖孔组合验证。

## 10. 验收标准

### 10.1 Android 原生层

- WebView 的实际 bounds 覆盖完整 Activity 可绘制窗口。
- `binding.main` 和 WebView 不再包含 systemBars 统一 padding。
- API 26～36 均保持一致的 edge-to-edge 意图。
- 旋转、分屏或导航方式变化后，CSS 变量在下一帧或合理短时间内更新。
- 相同 inset 不重复执行 JavaScript。
- 原生发布值与设备安全区误差不超过 1 CSS px。
- Android 深色系统模式下，SplashScreen 与浅色前端之间不出现黑色闪屏。

### 10.2 前端布局

- 状态栏背景自然显示移动顶栏或当前全屏页面背景。
- 手势导航区域可以显示页面背景，但固定按钮和最后一项内容不被遮挡。
- 三键导航保护层存在时，所有控件仍完全位于可交互区域。
- 竖屏和横屏下，挖孔不会遮挡标题、关闭按钮、输入框或主要操作。
- 不出现 `env()` 与 shell inset 相加导致的双重空白。
- 普通浏览器和桌面视口的布局没有新增空白或尺寸回归。
- 打开和关闭 Drawer、Dialog、图片预览、Notice 时不发生明显跳动。

### 10.3 视觉与交互

- 系统栏图标在当前浅色前端背景上具有清晰对比度。
- 状态栏和页面顶栏之间没有原生颜色断层。
- 手势导航栏背景与页面底部背景连续。
- 三键导航的系统半透明 scrim 被视为平台正常差异，不作为像素一致性缺陷。
- 软键盘弹出时表单仍可滚动和聚焦，且 IME 高度没有被永久写入安全区变量。

## 11. 验证矩阵

| 维度         | 最低覆盖                                                                               |
| ------------ | -------------------------------------------------------------------------------------- |
| Android 版本 | API 26、29/30、33/34、35、36                                                           |
| 导航模式     | 手势导航、三键导航                                                                     |
| 系统主题     | 浅色、深色（前端仍为浅色）                                                             |
| 屏幕方向     | 竖屏、横屏                                                                             |
| 屏幕形态     | 普通矩形、顶部挖孔、横屏侧边挖孔或模拟 cutout                                          |
| 窗口状态     | 全屏、分屏或可调整窗口（可用设备）                                                     |
| 输入状态     | 无键盘、键盘展开、输入框切换                                                           |
| 启动状态     | 冷启动、热恢复、页面刷新、前端加载失败兜底                                             |
| 业务状态     | 登录、运行设置、服务不可用、正常应用壳、Drawer、Dialog、Notice、图片预览、固定底部操作 |

前端桌面与浏览器回归仍按项目清单覆盖：

- `1440 × 900`。
- 接近 `768px` 的断点视口。
- `390 × 844` 移动视口。
- 真实 `getBoundingClientRect()`、计算样式和横向溢出。
- 浏览器控制台 error、warning 和 issue。

建议执行：

```powershell
Set-Location frontend
pnpm build

Set-Location ..\android
.\gradlew.bat :app:assembleDebug
```

设备 smoke 检查不能由构建通过替代。

## 12. 主要风险与应对

| 风险                             | 影响                     | 应对                                                                                     |
| -------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------- |
| 首次 inset 发布晚于页面首帧      | 顶栏短暂跳动             | CSS 默认零值；SplashScreen 保持到前端就绪；页面提交后重发缓存                            |
| Android px 直接当 CSS px         | 安全区成倍放大           | 按 density 转换并在 JS 中测量校验                                                        |
| WebView 同时提供非零 `env()`     | 双重避让                 | 统一变量取 `max()`，不得相加                                                             |
| 只处理 top/bottom                | 横屏挖孔遮挡             | 四边都发布并在关键容器消费                                                               |
| 把 IME 当普通 safe area          | 键盘收起后残留大空白     | 通用变量只含 systemBars 与 displayCutout；键盘继续由 `adjustResize`/visual viewport 处理 |
| 深色全屏浮层与深色系统栏图标冲突 | 图标对比度不足           | 第一版不让深色内容覆盖系统栏安全区；沉浸式需求另立桥方案                                 |
| 三键导航系统 scrim 改变颜色      | 无法像素级一致           | 作为平台行为接受，验收关注可读性和连续性                                                 |
| 页面刷新后 CSS 变量丢失          | 内容重新被遮挡           | 在页面提交可见/加载完成回调重新发布缓存                                                  |
| 文档与实现状态不一致             | 后续继续依赖过时架构描述 | 同一改动更新 Android、frontend、platforms 和代码地图                                     |

## 13. 回滚策略

本次不增加长期 feature flag。

如果设备验证发现无法在计划周期内修复的阻断问题：

1. 在同一开发分支回退本次 Android/fullscreen 与前端安全区提交。
2. 临时恢复根容器 systemBars padding。
3. 同时把 Android light/night 原生背景统一为前端浅色，避免恢复旧布局后重新出现深浅割裂。
4. 不保留一套运行时可切换的双实现，避免后续维护两个安全区模型。

## 14. 文档同步要求

本方案涉及的 Android WebView shell 已完成 edge-to-edge、CSS inset 发布与共享 Rust 服务集成；
API 36 手势导航和 API 33 三键导航已有真实设备记录，完整版本、窗口与输入状态矩阵仍按平台文档继续覆盖。

本次已同步：

- `docs/architecture.md` 的 Android 当前状态。
- `docs/platforms.md` 的 Android 当前状态和 edge-to-edge 职责。
- `docs/code-map/android.md` 的 inset publisher。
- `docs/code-map/frontend.md` 的统一安全区 foundation 层。
- `android/docs/README.md`。
- `frontend/docs/README.md` 及移动安全区规范入口。

本文件保留设计决策、风险和验收矩阵，不取代上述规范文档。

## 15. 推荐实施顺序

```text
1. 建立设备基线与截图
2. Android WebView 全屏 + WindowInsets 发布
3. 前端统一 safe-area 变量
4. AppShell / 登录 / 设置 / 服务异常状态
5. Dialog / Drawer / Notice / 图片预览
6. 各业务固定底部操作区与横屏 cutout
7. Android 版本与导航模式矩阵
8. 文档、注释、代码地图和失效代码清理
```

建议将 Android 基础设施与前端消费放在同一功能分支中完成，避免中间提交使 WebView 已全屏但前端尚未避让。提交记录可以按 Android 基础、frontend foundation、页面迁移、验证与文档拆分，便于审查和定位回归。

## 16. 参考资料

- Android Developers： [Display content edge-to-edge in views](https://developer.android.com/develop/ui/views/layout/edge-to-edge)
- Android Developers： [Behavior changes: Apps targeting Android 15 or higher](https://developer.android.com/about/versions/15/behavior-changes-15#edge-to-edge)
- Chrome for Developers： [Chrome on Android edge-to-edge migration guide](https://developer.chrome.com/docs/css-ui/edge-to-edge)
- 项目架构：`docs/architecture.md`
- 平台职责：`docs/platforms.md`
- Shell Bridge：`docs/shell-bridge.md`
- Android 代码地图：`docs/code-map/android.md`
- 前端代码地图：`docs/code-map/frontend.md`
- 前端视觉与一致性规范：`frontend/docs/visual-style.md`、`frontend/docs/ui-design-guidelines.md`、`frontend/docs/ui-consistency-checklist.md`

## 17. 本次实施记录

已完成：

- Android 保留 `enableEdgeToEdge()`，移除根容器 `systemBars` padding；
- 新增 `WebViewportInsetsPublisher`，完成四边 inset 采集、density 换算、受信任 origin 发布、去重和页面重发；
- 统一浅色前端对应的系统栏图标与 night 资源背景；
- 新增前端 `styles/foundation/_safe-area.scss`，迁移所有业务样式中的直接 `env(safe-area-inset-*)`；
- 补齐应用壳、认证、运行设置、服务不可用、Dialog、Drawer、Notice、图片预览和固定操作区的四边/底部安全区；
- 更新 Android、frontend、根平台文档和代码地图，并把未指定格式的报告默认规则写入 `AGENTS.md`。

已执行验证：

- 前端任务文件执行 Prettier 定向检查：通过；
- `pnpm build`（`frontend/`）：通过，包含 `vue-tsc -b` 与 Vite 生产构建；
- `.\gradlew.bat :app:testDebugUnitTest :app:assembleDebug :app:lintDebug --no-daemon`（`android/`，使用本机 JBR 21）：通过；
- 对最终 Debug APK 内 `assets/frontend` 与当前 `frontend/dist` 执行逐文件 SHA-256 对比，共检查 67 个文件，无缺失或内容不一致；
- Android 真实机 smoke：在 Xiaomi `25042PN24C`、Android 16 / API 36、手势导航设备上保留数据安装 Debug APK 并成功启动 `MainActivity`；
  - 竖屏和横屏下，UI hierarchy 均确认根容器与 WebView 覆盖完整物理窗口，分别为 `1440 × 3200` 和 `3200 × 1440`；
  - WindowManager 报告 `layoutInDisplayCutoutMode=always`，状态栏和导航栏均使用浅色背景对应的深色图标；
  - 竖屏 CSS 视口为 `384 × 853`，shell 发布 `top=45.07px`、`right=0`、`bottom=16px`、`left=0`；两个横屏方向的 CSS 视口均为 `853 × 384`，物理 cutout 分别正确更新为左侧或右侧 `45.07px`；三个方向均无横向溢出；
  - shell top 值与设备 `169px / 3.75 = 45.07px` 的物理 cutout/density 换算一致；WebView 自身 `env()` 为 `46px`，统一变量通过 `max()` 取值，没有双重相加；
  - 竖屏与两个横屏方向的真实截图确认状态栏、页面顶栏、底部固定操作区和手势导航区背景连续，按钮与文字未进入系统栏或 cutout 交互区域；
  - 通过 WebView CDP 强制重载页面后，四边 shell CSS 变量重新发布，未捕获 JavaScript exception、console error 或 warning；旋转回竖屏后左右 inset 也恢复为零；
- Android 真实机补充 smoke：在 Xiaomi `M2012K11AC`、Android 13 / API 33、三键导航设备上安装当前
  Debug APK；设备物理分辨率为 `1080 × 2400`、density 440；
  - 竖屏下登录、运行设置、服务不可用、正常应用壳、Drawer 和 Dialog 的标题、正文与操作区均未进入
    状态栏或三键导航保护区域；
  - 强制横屏后物理窗口为 `2400 × 1080`，WebView hierarchy 可见区域约为 `1200 × 540`；登录表单、
    按钮和底部内容均可达，未观察到明显裁切或横向溢出；
  - 旋转回竖屏、HOME 后热恢复和 force-stop 后冷启动均能重新得到可操作页面；
- Chrome DevTools MCP 桌面与移动回归：
  - `1440 × 900`、`768 × 900` 运行设置页无横向溢出；
  - `390 × 844` 注入 shell inset `top=32px`、`right=4px`、`bottom=24px`、`left=20px` 后，运行设置、AppShell、移动 Drawer、物品、库位和入库工作台均落在安全矩形内；
  - 入库工作台内容盒为 `366 × 708`，底部固定操作栏高度为 `89px`，其底部 padding 包含 `24px` 导航栏安全区，正文与操作栏不重叠；
  - `844 × 390` 注入左侧 cutout `36px`、底部 inset `24px` 后，运行设置工作区、普通 Dialog 与全屏网络工作区均未越过安全区；
  - 图片预览在 `top=32px`、`right=4px`、`bottom=24px`、`left=20px` 下按容器实际 padding 约束图片边界；
  - 上述检查的文档宽度均等于视口宽度，控制台无 error、warning 或 issue；
- 仍待覆盖 API 26、29/30、34、35 等版本，更多手势/挖孔组合、软键盘、分屏、深色系统主题和
  可调整窗口等完整设备矩阵。
