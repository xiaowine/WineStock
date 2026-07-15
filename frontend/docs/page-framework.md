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
- 当前原型里的库存列表、指标、物品编号、服务地址、按钮文案和操作项。

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
- 桌面和移动端使用同一棵应用框架 DOM；只有真实业务语义不同的列表呈现才允许拆分视图，业务页面、状态、API 调用和表单逻辑必须复用。
- 页面框架优先服务后台工具型工作流，但具体业务页面内容另行确认。
- 避免展示页式大边距、厚重卡片堆叠和营销化视觉。

## 桌面布局

桌面端采用类似 WinUI3 NavigationView 的结构：

```text
AppShell
  AppTopBar
  AppWorkspace
    AppNavigationPane
    AppContentPane
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
AppShell
  AppTopBar
  AppWorkspace
    AppContentPane
  AppNavigationPane  # 同一导航节点在移动端变为临时 Drawer
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
- 同一导航面板使用统一 motion token 从左侧滑入；关闭时等待面板原路退出后再恢复隐藏状态。
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
    _mixins.scss
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
- `foundation/`：颜色、字体、尺寸、圆角、阴影、层级、导航高度、motion token、基础元素、共享 SCSS 工具和减少动态效果适配。
- `shared/`：只放已经跨多个页面或组件复用的视觉原语，不接收单一业务页面的选择器。
- `styles/pages/_auth.scss`：认证页面族共享的布局和表单排列。
- `layouts/*.scss`：样式跟随所属 Shell，包含该 Shell 自己的断点调整。
- `components/**/*.scss`：样式与 Vue 组件同目录，组件状态和响应式规则由组件自身维护。
- `pages/*.scss`：业务页面表格、列表、分页和页面级状态由页面自身维护。

规则：

- 不再建立集中式 `responsive.scss`；断点规则放回对应布局、组件或页面文件。
- 组件样式使用同一套 token。
- 运行时主题值继续使用 CSS 自定义属性，不能用 Sass 变量替代。
- 结构断点、输入方式和图标骨架统一复用 `foundation/_mixins.scss`；组件文件通过 Sass `@use` 按需引入，不能依赖隐式全局变量。
- 重复状态族允许使用 Sass map、`@each` 和占位选择器生成；只有确实共享同一规则时才抽象，不能为了展示语法制造间接层。
- 同一 BEM 块的 `__element`、`--modifier`、伪类和直接子元素优先收进块选择器并使用 `&`；跨组件上下文和无父子语义的选择器保持显式平铺。
- SCSS 嵌套只用于明确的组件状态和子元素，避免按 DOM 结构进行深层嵌套。
- 无跨组件或插槽选择器依赖的新样式优先使用 scoped；需要外部上下文时必须让全局选择器的所有权清晰可追踪。
- 不把深色模式混入当前阶段。
- 不使用大面积装饰渐变、漂浮卡片和营销化 hero。

### SCSS 能力约束

- 新增或重构的 `.scss` 文件必须使用 Sass `@use` 声明依赖，禁止新增 `@import` 或依赖隐式全局变量。
- 同一 BEM 块的 `__element`、`--modifier`、伪类和直接子元素必须优先使用 `&` 嵌套；只有跨组件上下文、Teleport 宿主或没有父子语义的选择器才保留显式平铺。
- 断点、触控输入、焦点环、SVG 图标和减少动效规则必须优先复用 `foundation/_mixins.scss`；不得在多个组件重复书写相同的 `767px`、`768px` 媒体条件或图标骨架。
- 运行时可变的颜色、字体、圆角、层级和动效值继续使用 CSS 自定义属性；Sass 变量只用于编译期结构参数、断点和生成规则。
- 两个以上状态拥有同一结构时才使用 Sass map、`@each` 或占位选择器；抽象必须减少重复，不能为了展示 Sass 语法增加间接层。
- 嵌套只表达组件状态、BEM 关系和明确的直接子元素，禁止按任意 DOM 层级形成超过三层的深链选择器。
- 修改样式时不得把共享控件外观复制到业务页面；共享边框、焦点、禁用、错误和动效仍由共享层或组件所有者维护。

### SCSS 代码评审门槛

- 评审必须能从文件顶部的 `@use` 看出 mixin、Sass 内置模块和共享 token 的来源。
- 新增组件样式至少检查一次 `&__element`、`&--modifier` 或状态伪类是否可以替代重复的完整选择器；确实不能嵌套时在评审说明原因。
- 评审必须搜索变更文件中的 `@media`、`@import`、重复 CSS 值和深层选择器，确认没有绕过共享 mixin 或样式所有权。
- 生成 CSS 的视觉行为必须保持稳定；结构重构后仍需通过 `pnpm build`，并按 [`ui-consistency-checklist.md`](ui-consistency-checklist.md) 检查目标视口、溢出和状态。

## 当前源码对应

当前实现位置：

- `frontend/src/App.vue`：前端根路由出口；首次成功连接后始终保持路由树挂载，服务不可用时显示覆盖层，不销毁当前页面。
- `frontend/src/bootstrap/viewport.ts`：Vue 挂载后等待首帧布局，纠正移动 WebView 可能暂时使用的 980px 布局视口，并在业务路由挂载前完成媒体查询更新。
- `frontend/src/components/ServiceUnavailableScreen.vue`：服务未连接时的桌面与移动共用全屏覆盖提示，提供自动恢复说明和立即重试入口。
- `frontend/src/composables/useStablePendingIndicator.ts`：为初始服务检查和后续异步界面提供延迟显示、最短展示的防闪烁状态。
- `frontend/src/router/`：Vue Router 路由表、路由元数据和应用壳一级导航配置。
- `frontend/src/layouts/AppShell.vue`：已登录应用区域唯一的稳定应用框架；同一顶部栏、导航面板和路由出口通过 CSS 在桌面与移动端重排，移动端导航面板转为临时 Drawer。
- `frontend/src/components/AppNavigationList.vue`：桌面侧栏与移动 Drawer 共用的分组导航列表和线性图标。
- `frontend/src/components/RouteContentView.vue`：应用框架唯一的嵌套路由出口与页面切换动画；按路由身份重建页面，查询参数变化不强制销毁页面实例。
- `frontend/src/components/AccountUserSummary.vue`：应用框架顶部账户触发区与账户弹层复用的当前用户头像和名称展示。
- `frontend/src/components/AccountPopover.vue`：应用框架共用的紧凑账户信息与退出入口弹层。
- `frontend/src/composables/useShellLogout.ts`：应用框架共用的退出操作和反馈编排。
- `frontend/src/api/`：运行时服务地址、通用 HTTP 请求、统一错误和注册/登录接口契约。
- `frontend/src/auth/session.ts`：当前登录 token、用户摘要、启动恢复和串行 refresh 轮换。
- `frontend/src/auth/storage.ts`：纯 Web、Tauri WebView2 和 Android WebView 共用的版本化 localStorage refresh token 存储。
- `frontend/src/pages/LoginPage.vue`：桌面和移动共用的响应式登录表单、字段错误映射和登录成功导航。
- `frontend/src/pages/RegisterPage.vue`：桌面和移动共用的首个用户注册、密码确认、注册后自动登录和错误映射。
- `frontend/src/pages/ChangePasswordPage.vue`：桌面和移动共用的独立修改密码页面，强制改密时阻断其它前端页面。
- `frontend/src/pages/UsersPage.vue`：桌面表格和移动列表共用数据、筛选分页和管理操作状态。
- `frontend/src/pages/`：除鉴权页外还包含总览和物品正式页面入口；物品页使用全宽目录和共享编辑 Dialog，未实现的业务内容不展示假数据或开发说明。
- `frontend/src/styles/index.scss`：全局基础与共享视觉原语入口。
- `frontend/src/layouts/AppShell.scss`、`frontend/src/components/**/*.scss`、`frontend/src/pages/*.scss`：按 Vue 所有权拆分的稳定应用框架、组件和页面样式；桌面/移动断点规则归属实际拥有节点的样式文件。

路由与 history 策略见 `frontend/docs/routes.md`，HTTP 边界见 `frontend/docs/api-client.md`。
异步等待、后台刷新和错误恢复的稳定切换规则见 `frontend/docs/async-state-transitions.md`。

## 后续实施顺序

1. 为用户管理补自动化测试和更完整的移动账户操作。
2. 继续补齐物品管理的批量操作与自动化测试，并维持全宽目录和共享编辑 Dialog 的页面边界。
3. 在实际复用出现时继续抽取表格、分页和表单原语。
