# 前端页面框架

本文记录当前认可的前端页面框架。
它是后续实现 `frontend/` Shell、布局和样式层级时的当前准则。

## 确认范围

当前已经确认：

- 桌面端使用顶部栏 + 左侧常驻导航区域 + 右侧内容区域。
- 移动端使用顶部栏 + 单列内容区域 + 左侧临时导航 Drawer。
- 顶部栏不承担页面跳转。
- 桌面左侧导航区域和右侧内容区域是同级区域，不做卡片套卡片。
- 移动端模块导航使用 Drawer，页面级操作后续用 Sheet 或其它页面级容器。
- 桌面和移动端的当前用户头像统一位于顶部栏右侧；桌面同时显示名称，弹层只保留账户操作；移动端顶栏只保留头像，弹层再补全用户名和账户操作。
- 前端只实现浅色主视觉。

当前没有确认：

- 具体页面数量、路由路径、模块命名和排序。
- 每个页面的内容结构。
- 指标、表格、列表、字段和筛选项。
- 顶部栏、左侧导航、Drawer 中具体有哪些按钮。
- 库存、用户、库位等业务页面的最终交互流程。
- 当前原型里的库存列表、指标、SKU、服务地址、按钮文案和操作项。

总览和物品当前只保留正式页面入口，未实现的业务内容不会以假数据、假状态或开发说明出现在发布界面中。

## 所属边界

`frontend/` 是共享前端源码区域。
Desktop 和 Android 可以复用同一套前端源码，但构建产物由各自平台 shell 打包。

本文不定义：

- Tauri 窗口创建。
- Android Activity 或 WebView 生命周期。
- Axum 静态资源服务。
- 服务启动、停止、端口冲突和系统权限处理。

UI 通过 HTTP API 访问 `core` 暴露的服务。

## 总体原则

- 使用一套前端业务代码。
- 维护桌面和移动两种主要布局体验，而不是两套独立应用。
- 暂时只实现浅色主题。
- 桌面和移动端 Shell DOM 可以分开，业务页面、状态、API 调用和表单逻辑尽量复用。
- 页面框架优先服务后台工具型工作流，但具体业务页面内容另行确认。
- 避免展示页式大边距、厚重卡片堆叠和营销化视觉。

## 桌面布局

桌面端采用类似 WinUI3 NavigationView 的结构：

```text
DesktopShell
  DesktopTopNav
  MainViewport
    DesktopNavigationPane
    DesktopContentPane
```

### 顶部栏

顶部栏只负责产品标识以及后续确认的全局状态和少量快捷动作，不承担页面跳转。

当前包含：

- 产品标识。
- 顶部右侧的当前用户头像和名称；点击后显示账户操作弹层。

页面和模块跳转不放在顶部栏。
服务状态、服务地址和快捷入口只有在存在真实数据来源且交互确认后才能加入。

### 左侧导航面板

左侧导航面板是主容器的一列，和右侧内容区同级。
它不是主容器里的卡片，也不是临时浮层。

可承载的布局槽位：

- 一级模块跳转。
- 当前业务域的筛选入口。
- 批量操作、导出等快捷操作。
- 后续可能加入的树形导航、分类导航等工作区导航。

样式上使用浅底色和右边线表达区域边界，避免卡片套卡片。
当前桌面导航宽度为 `224px`；高频入口直接排列，管理入口使用轻分隔和分组标题，桌面与移动 Drawer 共用同一导航列表和线性图标。
左侧导航面板里的具体内容尚未确认。

### 右侧内容区

右侧内容区承载当前页面。
它应尽量使用可视区域，保持工作台密度。

右侧内容区可以预留这些布局槽位：

```text
ContentHeader
MetricStrip
Toolbar
DataRegion
```

指标区域和列表区域优先使用分隔线、表格、列表和工具栏表达层级。
不要把每一块都做成孤立卡片。
这些槽位只是页面框架层面的容器建议，不代表每个页面都必须有指标、工具栏或数据表。

## 移动布局

移动端不使用常驻左右分栏。
移动端采用单内容区加左侧临时导航 Drawer：

```text
MobileShell
  MobileTopBar
  MainViewport
  MobileNavigationDrawer
  MobileActionSheet
```

### 顶部栏

顶部栏承载：

- 左侧导航按钮。
- 当前页面标题。
- 右侧当前用户头像；点击后显示名称和退出入口的紧凑弹层。
- 搜索、新增等少量高频动作的位置。

顶部栏不放完整模块导航。
移动端顶部栏具体按钮尚未确认。

### 导航 Drawer

左侧 Drawer 用于模块跳转。
它和桌面左侧导航在认知上对应，但在移动端临时覆盖内容。

Drawer 打开时：

- 页面右侧压暗。
- 导航层整体淡入，Drawer 使用统一 motion token 从左侧滑入；关闭时等待面板原路退出后再移除导航层。
- 选择模块后关闭 Drawer。
- 不改变主内容滚动位置。

### 页面操作 Sheet

筛选、排序、更多操作等当前页面动作不放进全局导航 Drawer。
后续需要时使用底部 Sheet。

区分：

- 模块跳转：左侧 Drawer。
- 页面筛选和排序：底部 Sheet。
- 重内容编辑：全屏面板或独立页面。

Drawer、Sheet 和全屏面板中的具体内容由后续页面文档确认。

## 断点策略

先维护两种主要体验：

```text
mobile:  < 768px
desktop: >= 768px
```

可以针对平板补充局部调整，但不要引入第三套产品体验。

## 样式层级

视觉风格规则见 `frontend/docs/visual-style.md`。

当前样式层级：

```text
frontend/src/styles/
  index.scss
  foundation/
    _tokens.scss
    _base.scss
  shared/
    _brand.scss
    _controls.scss
    _forms.scss
    _page.scss
    _user-dialogs.scss
  pages/
    _auth.scss
frontend/src/layouts/*.scss
frontend/src/components/**/*.scss
frontend/src/pages/*.scss
```

职责：

- `index.scss`：使用 Sass `@use` 装配全局基础层，不导入具体页面或组件实现。
- `foundation/`：颜色、字体、尺寸、圆角、阴影、层级、导航高度、motion token、基础元素和减少动态效果适配。
- `shared/`：只放已经跨多个页面或组件复用的视觉原语，不接收单一业务页面的选择器。
- `styles/pages/_auth.scss`：认证页面族共享的布局和表单排列。
- `layouts/*.scss`：样式跟随所属 Shell，包含该 Shell 自己的断点调整。
- `components/**/*.scss`：样式与 Vue 组件同目录，组件状态和响应式规则由组件自身维护。
- `pages/*.scss`：业务页面表格、列表、分页和页面级状态由页面自身维护。

规则：

- 不再建立集中式 `responsive.scss`；断点规则放回对应布局、组件或页面文件。
- 组件样式使用同一套 token。
- 运行时主题值继续使用 CSS 自定义属性，不能用 Sass 变量替代。
- SCSS 嵌套只用于明确的组件状态和子元素，避免按 DOM 结构进行深层嵌套。
- 无跨组件或插槽选择器依赖的新样式优先使用 scoped；需要外部上下文时必须让全局选择器的所有权清晰可追踪。
- 不把深色模式混入当前阶段。
- 不使用大面积装饰渐变、漂浮卡片和营销化 hero。

## 当前源码对应

当前实现位置：

- `frontend/src/App.vue`：前端根路由出口；服务不可用时由全屏阻断层替换路由内容，不直接选择桌面或移动 Shell。
- `frontend/src/components/ServiceUnavailableScreen.vue`：服务未连接时的桌面与移动共用全屏提示，提供自动恢复说明和立即重试入口。
- `frontend/src/composables/useStablePendingIndicator.ts`：为初始服务检查和后续异步界面提供延迟显示、最短展示的防闪烁状态。
- `frontend/src/router/`：Vue Router 路由表、路由元数据和应用壳一级导航配置。
- `frontend/src/composables/useResponsiveShell.ts`：根据 `768px` 断点只挂载当前需要的 Shell，避免桌面和移动 Shell 同时渲染。
- `frontend/src/layouts/AppShell.vue`：已登录应用区域的响应式 Shell 选择入口。
- `frontend/src/layouts/DesktopShell.vue`：桌面顶部栏、顶部账户摘要、路由导航面板和嵌套路由内容区。
- `frontend/src/layouts/MobileShell.vue`：移动顶部栏、头像账户弹层、嵌套路由内容区和左侧路由导航 Drawer。
- `frontend/src/components/AppNavigationList.vue`：桌面侧栏与移动 Drawer 共用的分组导航列表和线性图标。
- `frontend/src/components/RouteContentView.vue`：桌面和移动共用的嵌套路由出口与页面切换动画，不重挂载应用壳。
- `frontend/src/components/AccountUserSummary.vue`：桌面顶部和移动账户弹层复用的当前用户头像与名称展示。
- `frontend/src/components/AccountPopover.vue`：桌面和移动共用的紧凑账户信息与退出入口弹层。
- `frontend/src/composables/useShellLogout.ts`：桌面与移动应用壳共用的退出操作和反馈编排。
- `frontend/src/api/`：运行时服务地址、通用 HTTP 请求、统一错误和注册/登录接口契约。
- `frontend/src/auth/session.ts`：当前登录 token、用户摘要、启动恢复和串行 refresh 轮换。
- `frontend/src/auth/storage.ts`：纯 Web、Tauri WebView2 和 Android WebView 共用的版本化 localStorage refresh token 存储。
- `frontend/src/pages/LoginPage.vue`：桌面和移动共用的响应式登录表单、字段错误映射和登录成功导航。
- `frontend/src/pages/RegisterPage.vue`：桌面和移动共用的首个用户注册、密码确认、注册后自动登录和错误映射。
- `frontend/src/pages/ChangePasswordPage.vue`：桌面和移动共用的独立修改密码页面，强制改密时阻断其它前端页面。
- `frontend/src/pages/UsersPage.vue`：桌面表格和移动列表共用数据、筛选分页和管理操作状态。
- `frontend/src/pages/`：除鉴权页外还包含总览和物品正式页面入口；未实现的业务内容不展示假数据或开发说明。
- `frontend/src/styles/index.scss`：全局基础与共享视觉原语入口。
- `frontend/src/layouts/*.scss`、`frontend/src/components/**/*.scss`、`frontend/src/pages/*.scss`：按 Vue 所有权拆分的布局、组件和页面样式。

路由与 history 策略见 `frontend/docs/routes.md`，HTTP 边界见 `frontend/docs/api-client.md`。
异步等待、后台刷新和错误恢复的稳定切换规则见 `frontend/docs/async-state-transitions.md`。

## 后续实施顺序

1. 为用户管理补自动化测试和更完整的移动账户操作。
2. 实现物品列表真实纵向功能，并沿用用户管理建立的请求和响应式页面边界。
3. 在实际复用出现时继续抽取表格、分页和表单原语。
