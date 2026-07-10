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
- 桌面端当前用户摘要固定在左侧导航底部，显示头像和名称，不占用顶部栏。
- 前端只实现浅色主视觉。

当前没有确认：

- 具体页面数量、路由路径、模块命名和排序。
- 每个页面的内容结构。
- 指标、表格、列表、字段和筛选项。
- 顶部栏、左侧导航、Drawer 中具体有哪些按钮。
- 库存、用户、库位等业务页面的最终交互流程。
- 当前原型里的库存列表、指标、SKU、服务地址、按钮文案和操作项。

除桌面登录和首个用户注册表单外，当前源码中的业务页面内容仍是预览占位，用来验证页面框架和视觉层级，不代表产品内容设计已经定稿。

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

顶部栏只负责全局状态和少量快捷动作，不承担页面跳转。

包含：

- 产品标识。
- 服务状态。
- 当前服务地址或连接状态。
- 搜索、通知等少量快捷入口位置。

页面和模块跳转不放在顶部栏。
顶部栏内的具体按钮和状态内容尚未确认。

### 左侧导航面板

左侧导航面板是主容器的一列，和右侧内容区同级。
它不是主容器里的卡片，也不是临时浮层。

可承载的布局槽位：

- 一级模块跳转。
- 当前业务域的筛选入口。
- 批量操作、导出等快捷操作。
- 后续可能加入的树形导航、分类导航等工作区导航。
- 底部固定的当前用户摘要，只读显示头像和名称，不提供页面跳转。

样式上使用浅底色和右边线表达区域边界，避免卡片套卡片。
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
- 搜索、新增等少量高频动作的位置。

顶部栏不放完整模块导航。
移动端顶部栏具体按钮尚未确认。

### 导航 Drawer

左侧 Drawer 用于模块跳转。
它和桌面左侧导航在认知上对应，但在移动端临时覆盖内容。

Drawer 打开时：

- 页面右侧压暗。
- Drawer 从左侧出现。
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

视觉风格规则见 `docs/frontend/visual-style.md`。

当前样式层级：

```text
frontend/src/styles/
  tokens.css
  base.css
  layout.css
  components.css
  responsive.css
```

职责：

- `tokens.css`：颜色、字体、尺寸、圆角、阴影、层级和导航高度。
- `base.css`：全局字体、基础元素、滚动和 focus 行为。
- `layout.css`：DesktopShell、MobileShell、主容器、导航区和 Drawer 布局。
- `components.css`：按钮、状态标记、指标、表格、列表和通用面板。
- `responsive.css`：断点显示和窄桌面微调。

规则：

- 断点文件只负责布局切换，不写两套完整视觉系统。
- 组件样式使用同一套 token。
- 不把深色模式混入当前阶段。
- 不使用大面积装饰渐变、漂浮卡片和营销化 hero。

## 当前源码对应

当前实现位置：

- `frontend/src/App.vue`：前端根路由出口，不再直接选择桌面或移动 Shell。
- `frontend/src/router/`：Vue Router 路由表、路由元数据和应用壳一级导航配置。
- `frontend/src/composables/useResponsiveShell.ts`：根据 `768px` 断点只挂载当前需要的 Shell，避免桌面和移动 Shell 同时渲染。
- `frontend/src/layouts/AppShell.vue`：已登录应用区域的响应式 Shell 选择入口。
- `frontend/src/layouts/DesktopShell.vue`：桌面顶部栏、路由导航面板和嵌套路由内容区。
- `frontend/src/layouts/MobileShell.vue`：移动顶部栏、嵌套路由内容区和左侧路由导航 Drawer。
- `frontend/src/components/SidebarUserSummary.vue`：桌面侧栏底部的当前用户头像和名称展示；当前没有点击行为。
- `frontend/src/api/`：运行时服务地址、通用 HTTP 请求、统一错误和注册/登录接口契约。
- `frontend/src/auth/session.ts`：当前登录 token、用户摘要、启动恢复和串行 refresh 轮换。
- `frontend/src/auth/storage.ts`：纯 Web、Tauri WebView2 和 Android WebView 共用的版本化 localStorage refresh token 存储。
- `frontend/src/pages/LoginPage.vue`：登录路由的响应式入口，桌面端使用真实表单，移动端仍为占位内容。
- `frontend/src/pages/login/DesktopLoginPage.vue`：桌面登录表单、字段错误映射和登录成功导航。
- `frontend/src/pages/RegisterPage.vue`：注册路由的响应式入口，桌面端使用首个用户表单，移动端只保留说明。
- `frontend/src/pages/register/DesktopRegisterPage.vue`：桌面首个用户注册、密码确认、注册后自动登录和错误映射。
- `frontend/src/pages/`：除鉴权页外还包含总览、物品和 404 页面骨架；当前不包含最终业务内容。
- `frontend/src/styles/`：页面框架样式层。

路由与 history 策略见 `docs/frontend/routes.md`，HTTP 边界见 `docs/frontend/api-client.md`。

## 后续实施顺序

1. 实现登出和当前 `requiresAuth` 元数据对应的鉴权守卫；强制改密的前端呈现方式另行确认。
2. 为首批页面分别补充页面文档，确认内容结构、字段、操作和移动端呈现方式。
3. 实现桌面物品列表代表页面，再单独确认移动端呈现。
4. 抽取稳定通用组件，再补充业务页面文档。
