# 深色模式设计与实现

本文定义 WineStock 前端浅色/深色主题的所有权、状态模型、SCSS token 结构、自动与手动切换、首屏呈现、平台联动和验收门槛。
它是主题实施的权威入口；通用视觉语言仍以 [`visual-style.md`](visual-style.md) 为准，控件和页面结构仍以
[`ui-design-guidelines.md`](ui-design-guidelines.md) 与 [`ui-consistency-checklist.md`](ui-consistency-checklist.md) 为准。

## 当前结论

- 当前源码已实现浅色与深色两套主题；`foundation/_tokens.scss` 通过 Sass map 单点输出运行时语义 token。
- 主题范围只包含浅色与深色，不开放自定义主色、任意配色或服务端账户同步。
- 用户偏好为三态：`跟随系统`、`浅色`、`深色`；默认值是 `跟随系统`。
- 主题是当前设备上的前端偏好，由 `frontend` 读取、持久化和立即应用，不进入 core HTTP API、共享 Rust 配置或 Shell Bridge 业务契约。
- SCSS 负责按主题输出语义 CSS 自定义属性；Vue/TypeScript 只管理偏好、系统媒体查询和平台副作用。业务组件不得判断当前主题后选择颜色。
- 深色模式已覆盖应用壳、认证/设置页、业务页面、共享控件、Dialog、Popover、Notice、图表和加载/错误状态；新增界面继续遵守本文的 token 与对比度门槛。

## 项目现状分析

### 已有基础

- `frontend/src/styles/foundation/_tokens.scss` 已集中定义页面、表面、文字、边框、主色、导航和状态色，大部分组件通过 `var(--color-*)` 消费，具备运行时换肤基础。
- `frontend/src/styles/index.scss` 是唯一全局 SCSS 装配入口，适合保持主题 token 先于基础与共享样式加载。
- `frontend/src/components/preferences/AppPreferencesDialog.vue` 已声明语言/主题属于本机偏好，并采用“即时生效并持久化”的交互边界。
- `frontend/src/main.ts` 在 Vue 挂载前完成应用装配，可初始化响应式主题状态；`frontend/index.html` 可承担脚本包执行前的首屏主题选择。
- Dashboard 趋势图主要使用 CSS 自定义属性，主题迁移成本较低。

### 主题迁移记录

实施前静态盘点在 `frontend/src` 的 `.scss`/`.vue` 中发现 `192` 处十六进制或 `rgb()` 颜色字面量，分布于 `45` 个文件。迁移已按以下两类完成；后续扫描不要求字面量为零，但新增值必须能归入同一所有权规则。

已迁移为语义 token：

- `AppShell.scss`、`_brand.scss`、`_controls.scss`、`_forms.scss` 中的白色表面、焦点环、悬浮底色和按钮前景。
- 各列表/表格中的固定列阴影、行悬浮色、选中边框、移动端粘性操作栏半透明白底。
- Dialog 遮罩、Popover/Notice 阴影、网络图画布表面与标签底色。
- `#fff` 形式的主按钮文字；深色主题会使用更亮的酒红强调色，前景必须改为 `--color-on-accent`，不能继续假定白字有足够对比度。
- 当前已被消费但没有在 foundation 定义的 `--color-surface-subtle`、`--color-success`、`--color-success-soft`。

允许保持固定，但必须在源码旁说明原因：

- `AttributeColorPicker.vue` 的 HSV/SV 光谱、色相渐变、预设业务颜色和拾色游标黑白描边；这些颜色表达被选择的真实颜色，不是应用主题。
- `BarcodeCameraView.vue` 的相机取景黑底、检测框和相机控件高对比前景；取景区在两种主题下都保持深色。
- `InAppImageViewer.scss` 的全屏图片遮罩与浅色控制图标；它是媒体查看环境，不随页面表面反转。
- 业务数据本身携带的颜色值。替代关系网络的内置节点调色板不是用户数据，仍需提供深色版本，不能归入该例外。

禁止直接用深色选择器覆盖组件，例如：

```scss
[data-theme="dark"] .some-page .some-card {
  background: #1f2328;
}
```

这种写法会把主题判断扩散到页面层。正确做法是组件继续使用语义 token，只有 foundation 输出两套 token 值。

## 状态模型

```ts
export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
```

状态含义：

| 状态            | 来源                                         | 行为                                               |
| --------------- | -------------------------------------------- | -------------------------------------------------- |
| `preference`    | 本机持久化值                                 | 用户选择的三态偏好                                 |
| `systemDark`    | `matchMedia("(prefers-color-scheme: dark)")` | 当前系统是否请求深色                               |
| `resolvedTheme` | `preference` + `systemDark`                  | 浏览器主题色、Android 系统栏等副作用使用的最终二态 |

解析规则固定为：

```ts
function resolveTheme(preference: ThemePreference, systemDark: boolean): ResolvedTheme {
  return preference === "system" ? (systemDark ? "dark" : "light") : preference;
}
```

- 存储键使用版本化名称 `winestock.theme.preference.v1`，值直接保存三态字符串，不需要为稳定枚举再包一层 JSON。
- 缺失、损坏、未知值或 `localStorage` 不可访问时回退 `system`，不得阻断应用启动。
- 设置偏好时先同步更新根元素，再更新响应状态与持久化，确保用户操作没有一帧延迟。
- 监听 `storage` 事件，使同一浏览器中的其它 WineStock 标签页同步；事件不得触发重复写回。
- 始终监听系统媒体查询，但只有 `preference === "system"` 时才改变生效主题。手动浅色/深色不能被系统切换覆盖。
- 运行时对外只暴露只读 `preference`、`resolvedTheme` 和显式 `setThemePreference()`；组件不得直接修改共享 ref。

## SCSS 结构

### 编译期与运行时分工

- Sass map、`@each` 和 mixin 只用于避免两套 token 声明重复结构。
- CSS 自定义属性承担运行时换肤，组件继续使用 `var(--color-*)`。
- 使用现有 `@use` 模块规则，不新增 `@import`。
- 不使用 `darken()`、`lighten()` 从浅色值推导深色值。两套颜色分别定义，保证实际对比度和层级可控。
- 尺寸、圆角、z-index 和 motion token 与主题无关，继续只在 `:root` 声明一次。

建议继续由 `foundation/_tokens.scss` 单点拥有 token，避免为了两张颜色表增加无业务价值的运行时样式层：

```scss
@use "sass:map";

$theme-light: (
  "color-page": #f4f5f7,
  "color-surface": #ffffff,
  "color-text": #17202a,
  "color-on-accent": #ffffff,
);

$theme-dark: (
  "color-page": #111316,
  "color-surface": #191c20,
  "color-text": #edf0f2,
  "color-on-accent": #271116,
);

@mixin emit-theme($theme) {
  @each $name in map.keys($theme) {
    --#{$name}: #{map.get($theme, $name)};
  }
}

:root,
:root[data-theme="light"] {
  @include emit-theme($theme-light);
  color-scheme: light;
}

:root[data-theme="dark"] {
  @include emit-theme($theme-dark);
  color-scheme: dark;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme]),
  :root[data-theme="system"] {
    @include emit-theme($theme-dark);
    color-scheme: dark;
  }
}
```

`data-theme` 表达用户偏好而不是最终二态：`system` 由媒体查询解析，`light`/`dark` 强制覆盖系统。根元素没有属性时也按系统媒体查询工作，作为首屏脚本失败或 JavaScript 被禁用时的 CSS 降级。

### Token 清单

第一版必须至少完整定义下列语义，不允许只为深色主题补页面背景：

| 语义                     | 浅色      | 深色      | 主要用途               |
| ------------------------ | --------- | --------- | ---------------------- |
| `--color-page`           | `#f4f5f7` | `#111316` | 页面与 WebView 根背景  |
| `--color-surface`        | `#ffffff` | `#191c20` | 主要内容表面           |
| `--color-surface-raised` | `#f7f8fa` | `#21252a` | 次级/抬升表面          |
| `--color-surface-subtle` | `#f1f3f5` | `#272c32` | 表头、弱分区、只读区   |
| `--color-text`           | `#17202a` | `#edf0f2` | 主文字                 |
| `--color-muted`          | `#59636f` | `#b3bbc4` | 辅助文字               |
| `--color-subtle`         | `#646e79` | `#8b95a1` | 最弱但仍需可读的文字   |
| `--color-border`         | `#dfe4e9` | `#343a42` | 普通分隔线             |
| `--color-border-strong`  | `#7f8993` | `#6e7984` | 控件边界和强分隔       |
| `--color-accent`         | `#6f2a36` | `#d48b98` | 主操作与选中态         |
| `--color-accent-strong`  | `#551f29` | `#e1a0ab` | 强调文字与 hover       |
| `--color-accent-soft`    | `#f4e9eb` | `#352429` | 低强调背景             |
| `--color-on-accent`      | `#ffffff` | `#271116` | 强调色填充上的前景     |
| `--color-teal`           | `#1d625b` | `#79c7bc` | 趋势图等既有青绿色语义 |
| `--color-teal-soft`      | `#edf6f4` | `#19312e` | 青绿色弱背景           |
| `--color-success`        | `#2e6b47` | `#78c993` | 业务成功               |
| `--color-success-soft`   | `#eaf4ed` | `#1b3124` | 成功弱背景             |
| `--color-warn`           | `#8a5a12` | `#e0b46c` | 警告                   |
| `--color-warn-soft`      | `#fbf1df` | `#352c1e` | 警告弱背景             |
| `--color-danger`         | `#9d2832` | `#ef8f96` | 错误与危险操作         |
| `--color-danger-soft`    | `#f9e6e8` | `#3a2225` | 错误弱背景             |

导航 token 继续独立保留，因为应用壳需要比内容区略深但不能与页面融成一片：

| 语义                 | 浅色      | 深色      |
| -------------------- | --------- | --------- |
| `--color-nav`        | `#f3f5f7` | `#15181c` |
| `--color-nav-raised` | `#ffffff` | `#1d2126` |
| `--color-nav-text`   | `#27313b` | `#dce1e5` |
| `--color-nav-muted`  | `#717b86` | `#9aa3ad` |
| `--color-nav-border` | `#d8dee5` | `#2e343b` |
| `--color-nav-active` | `#ffffff` | `#252a30` |

以上深色候选值的关键组合应达到 WCAG AA：正文/主表面约 `14.94:1`，辅助文字约 `8.81:1`，弱文字约 `5.63:1`，强调色/主表面约 `6.47:1`，深色主按钮前景/强调色约 `6.75:1`。最终验收仍以浏览器计算样式和实际对比度工具为准，不以表中理论值替代。

### 派生状态与阴影

行悬浮、选中、焦点环、遮罩、半透明粘性栏和阴影也会受主题影响。它们应提升为语义 token，或基于当前主题 token 使用项目已有的 `color-mix()`；不得继续混合固定的浅色 RGB 基色。

建议增加：

- `--color-row-hover`
- `--color-row-selected`
- `--color-focus-ring`
- `--color-overlay`
- `--color-surface-translucent`
- `--shadow-menu` 的深色值
- `--shadow-sticky`
- `--shadow-fixed-edge`

阴影在深色界面中只表达浮层和固定区边界，不通过大面积黑色模糊制造层级。普通页面区块仍优先使用背景差和边框。

## 运行时模块

建议新增以下前端所有权，不扩展 core、shared 或 Shell Bridge：

```text
frontend/src/theme/
  model.ts       # 类型、持久化值校验、resolveTheme 纯逻辑
  runtime.ts     # Vue 只读状态、matchMedia、storage、DOM 与平台副作用
frontend/tests/themeModel.test.mjs
```

`runtime.ts` 使用模块级单例状态，沿用 Vue `ref`/`computed`/`readonly` 的轻量共享状态方式，不引入新的状态管理依赖。`main.ts` 必须在任何 `await` 和 `createApp().mount()` 之前调用 `initializeTheme()`；初始化函数需幂等，便于 HMR 与测试。

公开面保持最小：

```ts
export const themePreference: Readonly<Ref<ThemePreference>>;
export const resolvedTheme: Readonly<Ref<ResolvedTheme>>;
export function initializeTheme(): void;
export function setThemePreference(value: ThemePreference): void;
```

初始化顺序：

1. 读取并校验持久化偏好。
2. 读取 `matchMedia` 当前值。
3. 同步设置 `document.documentElement.dataset.theme`。
4. 更新浏览器 `theme-color` 和平台系统栏基线。
5. 注册媒体查询与 `storage` 监听。
6. Vue 挂载后，偏好 Dialog 只消费只读状态并调用 setter。

主题切换不增加全局 `transition: background/color`。整页同时插值会造成大量重绘、文字短暂失去对比和首次加载闪烁；现有控件自己的 hover/浮层 motion 继续生效。

## 首屏与自动切换

仅在 Vue 挂载前初始化仍不足以彻底避免手动深色偏好下的浅色闪屏。`frontend/index.html` 需要在模块入口前放置一个最小同步脚本：

```html
<script>
  (() => {
    try {
      const value = localStorage.getItem("winestock.theme.preference.v1");
      if (value === "system" || value === "light" || value === "dark") {
        document.documentElement.dataset.theme = value;
      }
    } catch {
      // CSS 的 prefers-color-scheme 降级继续生效。
    }
  })();
</script>
```

- 脚本只承担首帧属性设置，不创建响应状态、不注册监听、不吞并 `runtime.ts` 的所有权。
- 键名和三态校验是 HTML/TypeScript 唯一允许的重复边界；自动化测试或构建检查需保证两边一致。
- 根元素无已保存属性时，SCSS 的 `prefers-color-scheme` 在 CSS 层直接选中系统主题，因此首次访问也不先闪浅色。
- `html`、`body`、`#app` 和视口稳定前的 `.app-viewport-bootstrap` 都必须使用 `--color-page`，避免启动阶段露出默认白底。
- `index.html` 应声明支持 `light dark` 的 color scheme；浏览器 `theme-color` 在初始化和系统切换后同步为 `--color-page` 的计算值。

系统主题在应用打开期间变化时：

- `system` 偏好由 CSS media query 立即更新 token。
- `matchMedia` 的 `change` 事件只更新 `resolvedTheme` 及浏览器/平台副作用。
- 手动 `light`/`dark` 保持不变；切回 `system` 时立即采用系统当前值。

## 偏好设置交互

入口复用账户弹层中的“偏好设置”和首次初始化向导最后一步，不新增独立主题页面或向导步骤。

`ThemePreferenceSelector.vue` 单点拥有三态单选/分段控件，`AppPreferencesDialog.vue` 与
`SetupWizardPage.vue` 都在“外观”分节复用它：

```text
外观
[跟随系统] [浅色] [深色]
```

- 默认选中 `跟随系统`，不显示额外教学文字或当前系统主题说明。
- 初始化向导把“外观”放在最后一页“数据收集”上方；选择即时应用并持久化，不等待运行配置 apply。
- 点击、方向键或 Space/Enter 选择后立即应用并持久化，不增加“保存”按钮；Dialog 的“关闭”只关闭浮层。
- 使用 `radiogroup`/`radio` 或等价原生单选语义，选中态不能只靠颜色表达。
- 控件复用全局边框、高度、焦点环、圆角和 motion token；不得做成三张说明卡片。
- 主题变化不得关闭 Dialog、重置遥测开关、抢夺焦点或改变面板尺寸。
- 偏好存储失败时仍在当前会话应用，不弹出阻断错误；下次启动按 `system` 回退。

## 浏览器与 Android 联动

### 浏览器原生控件

每个生效主题必须设置准确的 `color-scheme: light` 或 `dark`，让滚动条、日期/时间控件、输入自动填充等浏览器绘制部分与页面一致。不能只声明 `color-scheme: light dark` 后让浏览器自行选择，因为手动主题可能与系统相反。

需要单独复核 Chromium/WebView 自动填充背景、`datetime-local` 图标和原生文件选择入口；无法由 token 控制的部分优先使用 `color-scheme`，不复制浏览器私有样式。

### Android 系统栏

Android 的 `SystemBarAppearanceController` 在前端接管前按系统 night mode 选择栏图标；前端加载后由主题运行时发布基线，图片查看器不再固定恢复浅色。

Android Activity 只接管系统 day/night 的 `uiMode` 配置变化并原地刷新原生表面；切换系统主题时保留现有 WebView、Vue 实例、当前路由和页面内状态。宿主随后发送 `winestock:system-theme-refresh`，让主题 runtime 主动重读 media query，覆盖 WebView 未派发 `MediaQueryList change` 的实现差异；事件不携带主题值，最终状态仍由浏览器查询和用户偏好共同解析。旋转等其它配置变化不纳入这一例外，继续使用平台默认重建行为。

前端系统栏外观适配器继续使用现有 `WineStockSystemChrome.setDarkContent(boolean)`，不把视觉偏好扩展为 Shell Bridge 业务契约：

- 主题模块把 `resolvedTheme === "dark"` 作为系统栏基线。
- 图片查看器打开时申请临时深色内容覆盖；关闭、卸载或异常退出时释放覆盖并恢复当前主题基线，而不是固定恢复浅色。
- 适配器负责基线与临时覆盖计数，`InAppImageViewer.vue` 不再直接声明 `Window` 类型或猜测恢复值。
- Web/桌面没有该接口时保持 no-op。

Android 的 `values-night` 已提供深色 `web_background`、`android:isLightTheme=false` 和浅色系统栏图标配置，因此原生 Splash、Window、WebView 空白期及 WebView `prefers-color-scheme` 会跟随系统 day/night。WebView 算法着色保持关闭，实际页面颜色只由前端双主题 token 决定。手动主题属于 Web localStorage 偏好，原生层在 WebView 首帧前无法读取；若手动主题与系统相反，仍以首帧脚本接管为边界。

## 特殊视觉处理

- 趋势图继续消费 `--color-teal`、`--color-accent`、边框和表面 token；深色下复核折线、点、网格线与 tooltip，不在脚本中分支主题。
- 替代关系网络的七组内置节点颜色应升级为成对主题 token。组件只按索引设置 `var(--network-node-N-soft/strong)`，不得在 TypeScript 中维护第二张深色十六进制表。
- 用户或业务数据提供的颜色保持原值；必要时增加与当前表面的描边，不改变数据本身。
- 扫码取景与图片查看器保持固定深色环境，但它们的入口、Dialog 外壳和关闭后的页面必须正确恢复当前主题。
- 图片、logo 与 favicon 只有在深色背景下失去边界或可读性时才增加中性描边/底板；不得用 CSS filter 任意反转业务图片。

## 实施顺序

1. 增加三态模型、纯逻辑测试、首屏脚本和主题初始化；尚未完成 token 迁移前不对用户暴露手动入口。
2. 重构 `_tokens.scss` 为两套显式颜色 map，补齐缺失 token，并保证浅色计算样式与当前视觉基本一致。
3. 清理主题敏感颜色字面量，优先处理 foundation/shared、AppShell、Modal/Notice、认证与设置页，再覆盖各业务域和可视化。
4. 在初始化向导和偏好 Dialog 接入同一个三态控件，完成跨标签页、系统实时变化和存储失败降级。
5. 接入浏览器 `theme-color` 与 Android 系统栏基线/临时覆盖恢复。
6. 完成三档视口、两套主题、全状态浏览器验收后，才把深色主题视为可发布能力。

实施期间可以使用以下扫描确认没有新增未审计颜色：

```powershell
rg -n --glob '*.scss' --glob '*.vue' '#[0-9a-fA-F]{3,8}|rgba?\(' frontend/src
```

扫描结果不要求为零，但每个剩余字面量必须属于 foundation 主题表、真实业务颜色或上文列出的固定视觉，并能从源码注释判断所有权。

## 验收矩阵

复选框记录本次实现验收结果。Android 项已在 API 33 三键导航真机完成系统浅/深、手动浅/深、图片查看打开/关闭、Activity 恢复及 `uiMode` 原地切换验证；系统切换期间 PID、ActivityRecord、WebView target、路由和未提交页面状态均保持。其它 API 版本与手势导航仍属于发布设备矩阵。

### 行为

- [x] 无持久化值时，系统浅色/深色分别在首帧得到对应主题。
- [x] 系统为浅色时可手动深色，系统为深色时可手动浅色。
- [x] `system` 模式下切换操作系统主题，已打开页面立即更新且不重挂载 Vue 应用。
- [x] 手动模式下切换操作系统主题不改变页面；切回 `system` 立即采用当前系统值。
- [x] 刷新、重新打开、跨标签页和路由切换保持偏好与未保存业务上下文。
- [x] 损坏值、未知值、禁用存储和 `matchMedia` 不可用时安全回退，不阻断启动。
- [x] 切换主题不产生全局动画、白闪、页面尺寸变化、横向溢出或控制台错误。

### 视觉与可访问性

- [x] `1440 × 900`、接近 `768px`、`390 × 844` 三档视口均检查浅色和深色。
- [x] 应用壳、认证页、运行设置、全部业务页面、Dialog、嵌套 Dialog、Popover、Select 浮层、Notice 和服务不可用层均覆盖。
- [x] 默认、hover、active、focus-visible、selected、disabled、error、loading、空数据和遮罩状态均覆盖。
- [x] 普通文字对比度至少 `4.5:1`，大文字和非文字控件/焦点边界至少 `3:1`；禁用态不以牺牲可辨识性为代价。
- [x] 主按钮、危险按钮、状态弱背景、固定列、粘性栏和滚动条在两套主题下均可辨认。
- [x] 原生日期/时间/文件控件、自动填充、图表、网络图、图片查看和扫码取景无错误反色。

### 平台与工程

- [x] Web 浏览器 `color-scheme`、`theme-color` 与最终主题一致。
- [x] Android 状态栏/导航栏图标在浅色、深色、图片查看打开/关闭、Activity 恢复后均正确。
- [x] `themeModel` 测试覆盖三态解析、损坏存储回退、系统解析组合、关键色对比度和系统栏覆盖恢复。
- [x] `pnpm build`、相关 Node 测试、`pnpm format:check` 与 `git diff --check` 通过，无 Sass warning。
- [x] 浏览器控制台无新增 error、warning 或 issue；计算样式、`clientWidth`/`scrollWidth` 和关键控件坐标已实际读取。

## 技术依据

- Vue 当前指南允许使用独立于组件树的 `ref`/`computed` 共享简单应用状态，并建议以 `readonly()` 限制外部直接修改；watcher 适合执行 DOM/平台副作用：<https://vuejs.org/guide/scaling-up/state-management>、<https://vuejs.org/guide/essentials/watchers>。
- Sass 当前模块规则使用 `@use` 保证样式模块只装载一次；CSS at-rule 可直接承载媒体查询，Sass map/循环适合生成结构一致的 token：<https://sass-lang.com/documentation/at-rules/use/>。
