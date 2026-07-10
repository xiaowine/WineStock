# Frontend 代码地图

`frontend` 是 Vue/Vite 共享前端源码和 pnpm 工程。
它通过 HTTP 调用 core，不拥有平台 WebView 生命周期，Axum 也不能服务其构建产物。
未明确要求移动端时，当前界面改动默认只作用于桌面端。

## 工程入口

- `frontend/package.json`：Vue、Vite 和 Vue Router 依赖；安装和脚本统一使用 pnpm。
- `frontend/src/main.ts`：注册有效 token provider 和跨标签页同步、安装鉴权守卫、提前启动统一会话初始化并挂载 Vue Router。
- `frontend/src/App.vue`：前端根 `RouterView`，不拥有具体页面布局。
- `frontend/src/env.d.ts`：Vite 环境变量和平台运行时注入对象类型。

## 路由与布局

- `frontend/src/router/index.ts`：hash history、根应用壳嵌套路由、登录/注册路由和 catch-all 404。
- `frontend/src/router/meta.d.ts`：页面标题和 `requiresAuth` 元数据。
- `frontend/src/router/guards.ts`：等待会话初始化、拦截明确匿名访问、安全解析登录回跳，并在停留期间监听会话失效导航。
- `frontend/src/router/navigation.ts`：当前应用壳一级导航入口，不执行权限判断。
- `frontend/src/composables/useResponsiveShell.ts`：按 `768px` 断点只挂载当前桌面或移动 Shell。
- `frontend/src/layouts/AppShell.vue`：已登录应用区域的响应式 Shell 选择。
- `frontend/src/layouts/DesktopShell.vue`：桌面顶部会话/连接状态、左侧导航、当前用户摘要、真实登出按钮和嵌套路由出口。
- `frontend/src/layouts/MobileShell.vue`：移动顶部栏、Drawer 和嵌套路由出口；当前未接入真实登录界面。
- `frontend/src/components/SidebarUserSummary.vue`：桌面侧栏底部的只读用户头像和名称。

## API client 与鉴权状态

- `frontend/src/api/runtime-config.ts`
  - 解析平台注入的 `window.__WINESTOCK_RUNTIME_CONFIG__` 或 Vite 环境变量。
  - 校验 API 根地址必须为 HTTP/HTTPS，禁止把 `0.0.0.0` 作为访问地址。
  - 提供登录请求所需的客户端类型、设备名称和版本号。

- `frontend/src/api/client.ts`
  - 基于原生 `fetch` 实现统一 JSON 请求、查询参数、Bearer token 注入和 204 响应处理。
  - 只允许相对 API 路径，避免 access token 被发送到外部绝对地址。
  - 收到 `invalid_access_token` 时最多强制 refresh 并重试一次；不直接持久化 token，也不决定页面提示。

- `frontend/src/api/errors.ts`
  - 定义配置、网络、响应解析和非 2xx 错误类型。
  - 解析后端 `{ error: { code, message, details } }` 契约和字段校验详情。

- `frontend/src/api/auth.ts`
  - 定义注册、登录、refresh、logout、用户摘要和 token 响应 DTO。
  - 当前实现 `POST /api/auth/register`、`POST /api/auth/login`、`POST /api/auth/refresh` 和 `POST /api/auth/logout`。

- `frontend/src/auth/session.ts`
  - 在内存保存 access token、预计过期时间和用户摘要；refresh token 只从统一 localStorage 读取。
  - 公开五态会话初始化模型、单一初始化/refresh/logout Promise，并区分明确匿名和服务暂不可用。
  - 在跨标签页锁内读取最新 refresh token，执行轮换或服务端吊销；任何登出结果都会清除本地会话。
  - 监听其它同源标签页移除持久 token，并同步清除当前内存会话和进入匿名状态。

- `frontend/src/auth/coordination.ts`
  - 使用同一个 Web Locks API 锁在同源标签页和 Worker 间串行执行 refresh 与 logout。
  - Web Locks 不可用时直接执行任务，由会话层的最新 token 比较和单次重试兜底。

- `frontend/src/auth/storage.ts`
  - 使用版本化 `localStorage` 记录统一持久化 refresh token，并绑定获取 token 的 API 根地址。
  - 不保存 access token、密码或用户资料；损坏和不兼容记录会被清除。
  - 支持按预期 token 条件清除，并提供其它同源标签页移除记录的 `storage` 事件订阅。

## 页面

- `frontend/src/pages/LoginPage.vue`：登录路由的响应式入口；桌面端加载真实表单，移动端仍保留占位内容。
- `frontend/src/pages/login/DesktopLoginPage.vue`：桌面用户名密码表单，调用登录 API、映射字段错误、安全恢复内部目标并显示本机退出警告。
- `frontend/src/pages/RegisterPage.vue`：注册路由的响应式入口；桌面端加载首个用户表单，移动端只保留说明。
- `frontend/src/pages/register/DesktopRegisterPage.vue`：桌面首个用户注册、密码确认、错误映射和注册后自动登录流程。
- `frontend/src/pages/DashboardPage.vue`：总览页面骨架。
- `frontend/src/pages/ItemsPage.vue`：物品列表页面骨架。
- `frontend/src/pages/NotFoundPage.vue`：客户端路由 404。

## 样式和文档

- `frontend/src/styles/`：浅色 token、基础样式、布局、组件和响应式规则。
- `docs/frontend/page-framework.md`：页面框架和桌面/移动所有权。
- `docs/frontend/routes.md`：路由、history 策略和鉴权守卫状态。
- `docs/frontend/api-client.md`：API 地址、请求行为、错误契约和会话边界。
- `docs/frontend/auth-logout-and-route-guards.md`：登出 API/UI、会话状态、路由守卫、多标签页退出实现和验收记录。
- `docs/frontend/visual-style.md`：当前视觉规则。

## 平台边界

- `desktop/` 当前不是正式 Tauri shell，也不属于 Cargo 工作区。
- 正式 Desktop/Android shell 应在前端挂载前注入运行时 API 地址和客户端元数据。
- 不要从当前脚手架推断最终平台包名、WebView 协议或资源目录。
