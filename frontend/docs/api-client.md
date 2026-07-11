# 前端 API Client

本文记录 `frontend/src/api/` 的运行时地址、请求行为、错误契约和当前鉴权会话边界。

## 所有权

API client 属于共享前端 HTTP 边界，负责：

- 解析当前可访问的服务根地址。
- 构造查询参数和 JSON 请求。
- 在需要时附加 Bearer access token。
- 解析 204、JSON、文本和后端统一错误响应。
- 把字段级校验详情转换为页面可消费的字段错误。

API client 不负责：

- 启动、停止或发现 Axum 服务。
- 决定 Desktop 或 Android WebView 生命周期。
- 直接读写持久化 token。
- 页面导航。
- 把 `0.0.0.0` 转换为用户访问地址。

服务可用性由 `frontend/src/service/availability.ts` 通过无鉴权 `GET /api/health` 独立探测。该监控在服务断开时让根应用显示全屏提示，而不是把断连状态混入某个业务页面；服务恢复后会触发会话恢复并自动返回原路由。

## 契约核对

新增、删除或判断前端 API 能力时，优先读取当前运行服务的 `/api-docs/openapi.json`，确认路径、方法、查询参数、请求体和响应结构。领域文档用于补充业务语义，后端源码只用于追踪实现或排查契约与实际行为不一致。

不能仅根据前端现有调用、手写 DTO 或页面是否展示某个控件来判断后端接口不存在。删除搜索、筛选或其它 API 相关交互前，必须先完成 OpenAPI 核对。

## API 根地址

地址解析优先级：

1. 平台挂载 Vue 应用前注入的 `window.__WINESTOCK_RUNTIME_CONFIG__.apiBaseUrl`。
2. Vite 环境变量 `VITE_API_BASE_URL`。

当前不提供硬编码 IP、端口或静默默认地址。
缺少地址时请求会抛出 `ApiConfigurationError`，由页面显示明确配置提示。

服务地址必须：

- 使用 `http` 或 `https`。
- 使用客户端真实可访问的 host。
- 不包含用户名、密码、查询参数或 hash。
- 不能使用只用于绑定的 `0.0.0.0`。

平台运行时注入结构：

```ts
window.__WINESTOCK_RUNTIME_CONFIG__ = {
  apiBaseUrl: 'http://<service-host>:<port>',
  clientKind: 'desktop',
  deviceName: '<device-name>',
  appVersion: '<app-version>',
}
```

本地 Vite 开发可在不提交到仓库的 `.env.local` 中设置对应 `VITE_*` 变量。

## 请求行为

`ApiClient.request<T>()` 当前支持：

- `GET`、`POST`、`PUT`、`PATCH`、`DELETE`。
- 标量和数组 query 参数。
- JSON 请求体与 `Content-Type: application/json`。
- 默认 `Accept: application/json`。
- 可选 `AbortSignal`。
- 成功响应按需读取为鉴权保护的 `Blob`。
- 204 空响应。
- access token provider 注入。
- 网络连接失败通知函数注入；主动取消请求不会触发通知。
- `invalid_access_token` 时通过 provider 强制 refresh 并最多重试一次。

`ApiClient.upload<T>()` 使用 `XMLHttpRequest` 发送 multipart，从而提供浏览器原生上传进度；它仍复用同一个 access token provider、`invalid_access_token` 强制 refresh 和统一 `ApiError` 契约。页面不得自行从持久化存储读取 token。

请求路径必须以 `/` 开头且作为当前根地址的相对 API 路径处理，调用方不能传入外部绝对 URL，以免 Bearer token 泄漏到其它 host。

## 错误类型

- `ApiConfigurationError`：运行时配置或请求参数无效。
- `ApiNetworkError`：浏览器无法建立连接或完成请求。
- `ApiResponseError`：成功响应声明为 JSON 但内容无法解析。
- `ApiError`：非 2xx HTTP 响应。

`ApiError` 解析后端稳定契约：

```json
{
  "error": {
    "code": "invalid_request",
    "message": "请求参数无效",
    "details": null
  }
}
```

当 `details.kind = "validation"` 时，`details.fields` 会按字段路径聚合到 `ApiError.fieldErrors`。
页面业务分支应优先判断稳定 `code`，不解析 message 文本。

## 鉴权会话

`frontend/src/auth/session.ts` 当前在内存中保存：

- access token。
- access token 预计过期时间。
- 当前用户和权限摘要。

统一持久化规则：

- access token 不持久化，只在内存中使用。
- refresh token 由 `frontend/src/auth/storage.ts` 使用版本化 `localStorage` 记录。
- 记录包含 API 根地址，切换服务时不会把旧 token 发送到新服务。
- 纯 Web、Tauri WebView2 和 Android WebView 使用同一存储机制，不需要平台桥或 Cookie。
- localStorage 中的 refresh token 可被同源 JavaScript 读取，因此必须配合严格 CSP、受控资源和 refresh token 轮换。
- 会话公开 `idle`、`restoring`、`authenticated`、`anonymous` 和 `unavailable` 状态，路由只把明确 `anonymous` 视为未登录。
- 桌面端已经提供真实登出入口，全局路由守卫已经接入。

登出 API、会话初始化状态和路由守卫的完整实现见 [`auth-logout-and-route-guards.md`](auth-logout-and-route-guards.md)。

## 会话恢复与轮换

- 初始路由导航等待统一初始化 Promise；持久 token 存在时调用 `POST /api/auth/refresh`。
- access token 临近过期时，首个业务请求先执行 refresh。
- 并发请求共用同一个 refresh Promise，避免重复使用已经轮换失效的旧 token。
- 支持 Web Locks 时，同源标签页和 Worker 共用独占 refresh 锁；获得锁后才读取 localStorage 中的最新 token。
- Web Locks 不可用或仍发生竞态时，旧 token 被拒绝后会重新读取持久记录；记录已更新则使用新 token 重试一次。
- refresh 成功后先覆盖保存新 refresh token，再更新内存会话。
- `invalid_refresh_token` 只有在失败 token 仍是当前持久记录时才会清除 localStorage，避免旧标签页删除其它标签页的新 token。
- 一个标签页移除持久 token 时，其它同源标签页通过 `storage` 事件清除内存会话。
- 网络或服务暂时不可用时进入 `unavailable`，不删除持久化 token，也不把受保护页面误重定向为凭据失效。

## Access token 自动刷新

- `frontend/src/auth/auto-refresh.ts` 根据内存中的 `accessTokenExpiresAt` 使用一次性定时器调度，不进行固定频率轮询。
- 默认在 access token 到期前约 50 至 60 秒 refresh；每个标签页使用最多 10 秒抖动，降低同时抢锁的概率。
- 自动刷新仍调用 `refreshAuthSession()`，因此复用同标签页 Promise、最新持久 token 读取和跨标签页 Web Locks 锁。
- 网络、配置或响应失败后保持 `unavailable` 和持久 token，并在约 30 秒后自动重试。
- 窗口重新获得焦点、页面恢复可见或浏览器触发 `online` 时立即补检，弥补后台标签页定时器被节流的情况。
- 登出开始、明确匿名或尚未建立会话时取消定时器；自动刷新不会让已退出会话重新登录。

## 服务可用性监控

- 应用启动后立即调用 `GET /api/health`，即使本地没有 refresh token 也能识别服务未连接。
- 服务可用时每 15 秒检查一次，不可用时每 5 秒重试一次；单次检查 4 秒超时，且不会并发发送重复探测。
- 窗口重新聚焦、页面恢复可见或浏览器触发 `online` 时立即补检；隐藏页面暂停定时轮询。
- 断连时根应用使用全屏状态层替换路由内容；用户也可以手动立即重试。
- 任意业务 API 发生真实网络连接失败时会立即进入全屏断连状态，不必等待下一次定时健康检查；请求取消不会触发。
- 健康检查恢复后，如果鉴权会话处于 `unavailable`，会立即复用统一会话初始化入口恢复 token；完成后显示原路由。

## 登出

- `POST /api/auth/logout` 只提交获得跨标签页锁后重新读取的最新 refresh token，不依赖 Bearer access token。
- refresh 与 logout 使用同一个 Web Locks 独占锁，避免只吊销已经被其它标签页轮换的旧 token。
- 服务端返回 204 时结果为 `revoked`；`invalid_refresh_token` 或本地已无 token 时结果为 `already_invalid`。
- 网络、配置、5xx 或响应错误时结果为 `local_only`；无论哪种结果，本机会清除内存和持久会话。
- 同标签页重复登出复用一个 Promise；登出期间 API client 不再启动新的 refresh。
- 其它标签页收到持久 token 移除事件后清除内存 access token，并由路由监听离开受保护页面。

## 当前鉴权与用户管理接入

- 响应式注册页不携带 access token 调用 `POST /api/auth/register` 创建首个用户，避免该公开入口误用于创建后续用户。
- 首个用户注册成功后使用相同凭据调用登录接口，建立内存会话并导航到总览。
- 服务已有用户时，未登录注册会提示返回登录页；用户管理页使用当前会话调用同一接口创建后续用户。
- 响应式登录页调用 `POST /api/auth/login`。
- 登录请求的设备名称、客户端类型和版本号来自运行时配置。
- 字段校验错误映射到用户名和密码输入框。
- 登录成功建立内存会话；合法内部 `redirect` 使用 replace 回到原目标，其它值回到总览。
- 登录和首个用户注册在桌面与移动视口共用同一表单和业务流程。
- 登录响应始终先建立受管理会话；服务端返回 `password_change_required` 时，由全局守卫进入独立修改密码页。
- 修改密码页调用 `POST /api/auth/me/password`，请求包含当前密码和新密码，成功响应为 204。
- 强制改密期间前端只允许进入修改密码页；后端也只允许访问 `/api/auth/me` 和 `/api/auth/me/password`。
- 修改成功后保留现有 token 和权限，只清除当前会话用户摘要中的强制改密标记并恢复原内部目标。

用户管理 API 集中在 `src/api/users.ts`，已接入用户分页查询、启停、权限整体替换、临时密码和权限定义。列表筛选变化时使用 `AbortSignal` 取消旧请求，页面按稳定错误代码处理权限不足、用户不存在、权限不存在和防锁死冲突。
