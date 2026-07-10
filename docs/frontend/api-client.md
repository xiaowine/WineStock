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
- 204 空响应。
- access token provider 注入。
- `invalid_access_token` 时通过 provider 强制 refresh 并最多重试一次。

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

## 登出

- `POST /api/auth/logout` 只提交获得跨标签页锁后重新读取的最新 refresh token，不依赖 Bearer access token。
- refresh 与 logout 使用同一个 Web Locks 独占锁，避免只吊销已经被其它标签页轮换的旧 token。
- 服务端返回 204 时结果为 `revoked`；`invalid_refresh_token` 或本地已无 token 时结果为 `already_invalid`。
- 网络、配置、5xx 或响应错误时结果为 `local_only`；无论哪种结果，本机会清除内存和持久会话。
- 同标签页重复登出复用一个 Promise；登出期间 API client 不再启动新的 refresh。
- 其它标签页收到持久 token 移除事件后清除内存 access token，并由路由监听离开受保护页面。

## 当前注册与登录接入

- 桌面注册页不携带 access token 调用 `POST /api/auth/register` 创建首个用户，避免该公开入口误用于创建后续用户。
- 首个用户注册成功后使用相同凭据调用登录接口，建立内存会话并导航到总览。
- 服务已有用户时，未登录注册会提示返回登录页；后续用户管理注册入口尚未实现。
- 桌面登录页调用 `POST /api/auth/login`。
- 登录请求的设备名称、客户端类型和版本号来自运行时配置。
- 字段校验错误映射到用户名和密码输入框。
- 登录成功建立内存会话；合法内部 `redirect` 使用 replace 回到原目标，其它值回到总览。
- 移动端登录页面仍为占位内容，注册路由只显示桌面端完成提示。
- 服务端返回 `password_change_required` 时当前停留在登录页并明确提示，相关 UI 流程尚未确定。
