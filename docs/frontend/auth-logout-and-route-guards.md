# 前端登出与鉴权路由守卫实现方案

本文记录 WineStock 前端“真正登出”、会话初始化和鉴权路由守卫的已实现方案与验收要求。
实现建立在 API client、localStorage refresh token、Web Locks 多标签页轮换和 Vue Router 路由骨架之上，不改变 Axum 与平台 shell 的所有权边界。

## 实现状态

- `POST /api/auth/login` 和 `POST /api/auth/refresh` 已接入前端。
- `POST /api/auth/logout` 已接入会话层，成功时吊销提交的最新 refresh token。
- access token 只保存在内存中，refresh token 使用版本化 localStorage 持久化。
- 同标签页 refresh/logout 各自共用 Promise；同源标签页通过同一个 Web Locks 锁串行轮换和吊销。
- 一个标签页移除持久 token 时，其它标签页会同步清除内存会话并离开受保护页面。
- 会话层公开五态初始化模型，网络不可用和凭据失效不会混为一谈。
- `meta.requiresAuth` 已由全局异步前置守卫执行。
- 桌面应用壳已经提供登出入口、进行状态和本机退出警告。

## 实现目标

本次实现需要达到：

1. 用户点击退出时，优先吊销当前服务端 refresh token。
2. 无论服务端登出是否成功，本机都必须清除内存和持久会话。
3. 一个标签页退出时，其它同源标签页同步退出。
4. 未登录用户不能进入 `requiresAuth = true` 的页面。
5. 启动时必须等待持久会话恢复完成，再判断是否重定向。
6. 登录成功后返回用户原本要访问的内部页面。
7. refresh token 真正失效时，停留在受保护页面的用户自动进入登录页。
8. 网络暂时不可用时不把“会话状态未知”错误判断为“明确未登录”。

## 不在本次范围

- 不改变服务端 JWT 和 refresh token 轮换策略。
- 不让 Axum 服务前端构建产物。
- 不新增 Cookie、平台桥或平台专用 token 存储。
- 不实现权限代码与所有页面的完整映射。
- 不实现 `password_change_required` 对应的改密页面。
- 不确定移动端最终账户菜单样式；会话和守卫逻辑必须保持共享。

## 所有权边界

| 边界 | 负责内容 | 不负责内容 |
| --- | --- | --- |
| `frontend/src/api/auth.ts` | logout HTTP DTO 和请求函数 | 本地 token 清理、导航、页面提示 |
| `frontend/src/auth/session.ts` | 会话状态、初始化、服务端登出用例、本地清理 | Vue Router 导航、按钮布局 |
| `frontend/src/auth/coordination.ts` | refresh 与 logout 的跨标签页互斥 | token 持久化、HTTP 请求 |
| `frontend/src/auth/storage.ts` | refresh token 读取、条件清理和 storage 事件 | 服务端吊销、路由跳转 |
| `frontend/src/router/guards.ts` | 全局守卫、登录回跳和会话失效导航 | token 解析、HTTP 业务 |
| 桌面应用壳 | 退出按钮、加载状态和用户提示 | 服务端 token 规则 |
| `core` auth 模块 | 吊销提交的 refresh token | 前端导航和 localStorage |

服务端现有 logout 行为保持不变：只吊销 refresh token；已签发的 JWT access token 按短 TTL 自然过期。

## 会话状态模型

仅用 `authSession.value === null` 无法区分“尚未恢复”和“已经确认未登录”。
会话层公开只读状态：

```ts
export type AuthStatus =
  | 'idle'
  | 'restoring'
  | 'authenticated'
  | 'anonymous'
  | 'unavailable'
```

状态含义：

| 状态 | 含义 | 路由行为 |
| --- | --- | --- |
| `idle` | 尚未开始读取持久会话 | 守卫触发初始化 |
| `restoring` | 正在调用 refresh 恢复 | 守卫等待同一个初始化 Promise |
| `authenticated` | 内存中存在有效会话 | 允许受保护路由 |
| `anonymous` | 无持久 token，或 token 已明确失效 | 受保护路由重定向登录页 |
| `unavailable` | 服务地址、网络或响应暂时不可用，持久 token 仍保留 | 不误判为未登录；允许保留目标路由并显示连接异常状态 |

主要状态转换：

```text
idle -> restoring -> authenticated
                  -> anonymous
                  -> unavailable

login success -> authenticated
logout        -> anonymous
invalid refresh token -> anonymous
other tab removes token -> anonymous
unavailable retry success -> authenticated or anonymous
```

## 启动初始化

会话层通过 `ensureAuthSessionInitialized()` 复用单一初始化 Promise：

```ts
export function ensureAuthSessionInitialized(): Promise<AuthStatus>
```

行为要求：

- 已处于 `authenticated` 或 `anonymous` 时直接返回当前状态。
- `restoring` 时返回已有 Promise，不重复 refresh。
- `idle` 或用户主动重试 `unavailable` 时开始恢复。
- 没有持久 refresh token 时进入 `anonymous`。
- refresh 成功时进入 `authenticated`。
- `invalid_refresh_token` 时进入 `anonymous`，并按现有条件清理规则删除失效记录。
- 网络、配置或响应格式错误时进入 `unavailable`，不得删除持久 token。

`main.ts` 可以继续提前调用初始化以减少等待，但路由守卫必须等待同一个 Promise，不能依赖“已经发起但没有 await”的时序。

## 前端 logout API

`frontend/src/api/auth.ts` 已提供：

```ts
export interface AuthLogoutRequest {
  refresh_token: string
}

export function logout(request: AuthLogoutRequest): Promise<void>
```

请求约束：

- 路径为 `POST /api/auth/logout`。
- 请求体只包含当前 refresh token。
- 设置 `authenticated: false`，因为该接口不依赖 Bearer access token。
- 204 响应由现有 API client 解析为空返回值。
- 不记录、打印或放入 URL/query 的 token。

## 会话层登出用例

会话层已提供：

```ts
export type LogoutResult = 'revoked' | 'already_invalid' | 'local_only'

export function logoutAuthSession(): Promise<LogoutResult>
```

同时公开只读 `isLoggingOut`，供按钮禁用和加载文案使用。
`isLoggingOut` 变为 true 后，`getValidAccessToken()` 不得再启动新的 refresh，避免退出过程中出现新的业务鉴权请求与 logout 竞争。

### 执行顺序

```text
设置 logging-out 状态
-> 等待同标签页已有 refresh 结束
-> 获得跨标签页 auth refresh 锁
-> 在锁内读取最新持久 refresh token
-> 调用 POST /api/auth/logout
-> finally 清除内存和 localStorage
-> storage 事件通知其它标签页清除内存会话
-> 返回登出结果供 UI 决定提示
```

logout 必须与 refresh 使用同一个 Web Locks 锁名，避免下面的竞态：

```text
标签页 A 读取旧 token 准备 logout
标签页 B 同时 refresh 并写入新 token
标签页 A 只吊销旧 token，新 token 仍然有效
```

获得锁后重新读取 localStorage，可以保证支持 Web Locks 的环境吊销最新 token。

### 无 Web Locks 环境

无锁环境按现有 refresh 兜底思路处理：

1. 读取 token 并请求 logout。
2. 如果返回 `invalid_refresh_token`，重新读取 localStorage。
3. 如果记录已经变化，使用最新 token 最多重试一次。
4. 无论最终结果如何，都清除本地会话。

### 错误语义

| 情况 | 本地处理 | `LogoutResult` | UI 建议 |
| --- | --- | --- | --- |
| 服务端返回 204 | 清除本地会话 | `revoked` | 正常进入登录页 |
| token 已不存在或返回 `invalid_refresh_token` | 清除本地会话 | `already_invalid` | 视为已退出，不显示错误 |
| 网络、配置、5xx 或响应错误 | 清除本地会话 | `local_only` | 进入登录页并提示服务端吊销未确认 |

用户明确点击退出时，即使网络失败也不能保留本地 token，否则用户会被迫保持登录。
`local_only` 表示服务端 refresh token 可能仍有效至过期，不代表本机仍保持登录。

## 登出 UI

当前实现范围先完成桌面端入口：

- 在桌面侧栏当前用户摘要附近增加“退出登录”按钮。
- 点击后立即执行，不增加确认弹窗。
- 请求期间禁用按钮并显示“正在退出…”。
- 完成后使用 `router.replace({ name: 'login' })`，避免后退重新显示受保护页面。
- `local_only` 时登录页显示固定提示：“本机已退出，但服务端会话吊销未确认”。
- UI 不显示后端原始 token、请求体或敏感错误详情。

第一版可以由 `DesktopShell.vue` 组织退出操作；如果账户菜单继续增加，再抽取独立账户面板组件。
移动端仍保留当前页面范围，但未来入口必须调用同一个 `logoutAuthSession()`，不得实现平台专用 token 清理逻辑。

## 全局路由守卫

`frontend/src/router/guards.ts` 安装全局前置守卫和会话失效监听。
会话层不得反向导入 Router。

### 前置守卫

伪代码：

```ts
router.beforeEach(async (to) => {
  const status = await ensureAuthSessionInitialized()

  if (to.meta.requiresAuth && status === 'anonymous') {
    return {
      name: 'login',
      query: { redirect: to.fullPath },
    }
  }

  if (to.name === 'login' && status === 'authenticated') {
    return { name: 'dashboard' }
  }
})
```

守卫要求：

- 可以异步等待初始化。
- 只在明确 `anonymous` 时把受保护页面重定向到登录页。
- `unavailable` 不重定向登录页，避免把服务暂时不可用误报为凭据失效。
- 登录页自身不能再次重定向到登录页，避免死循环。
- 注册页继续保持公开；服务端仍负责限制“首个用户”或后续注册权限。
- 404 页面保持公开。

### 登录后回跳

未登录访问 `/items` 时保存内部目标：

```text
/login?redirect=/items
```

登录成功后：

1. 只接受字符串形式的内部路径。
2. 目标必须以 `/` 开头，不能包含外部 scheme 或 host。
3. 使用 `router.resolve()` 确认路由可解析。
4. 无合法目标时进入 `dashboard`。
5. 使用 `router.replace()`，避免登录页留在历史栈中。

不能把用户名、token、密码或 API 根地址写入 redirect query。

## 停留页面期间的会话失效

`beforeEach` 只在发生导航时运行，不能处理用户一直停留在页面中的情况。
`router/guards.ts` 还需要监听会话状态：

```ts
watch(authStatus, (status) => {
  if (status === 'anonymous' && router.currentRoute.value.meta.requiresAuth) {
    void router.replace({
      name: 'login',
      query: { redirect: router.currentRoute.value.fullPath },
    })
  }
})
```

这个监听覆盖：

- 其它标签页退出并移除 localStorage。
- API client 强制 refresh 后确认 refresh token 失效。
- 当前标签页主动清除会话。

监听和前置守卫需要共用一个重定向辅助函数，避免重复导航和无限循环。

## 多标签页行为

### 一个标签页退出

```text
标签页 A 获得 auth 锁并吊销最新 refresh token
-> A 删除 localStorage
-> B 收到 storage 事件并清除内存会话
-> B 的 authStatus 变为 anonymous
-> B 的路由监听跳转登录页
```

### 退出与 refresh 同时发生

- 支持 Web Locks 时，两者使用相同独占锁，按顺序执行。
- logout 获得锁后重新读取最新 token。
- logout 删除记录后，后续等待的 refresh 读取不到 token，直接返回匿名状态。

### access token 注意事项

服务端 logout 当前不会立即撤销已经签发的 JWT access token。
其它标签页收到 storage 事件后会清除内存 access token 并离开受保护页面，但已经发出的请求可能完成；这是当前短 TTL JWT 设计的预期行为。

## 路由守卫不是安全边界

前端守卫只负责导航体验，不能代替服务端鉴权：

- 用户可以修改前端代码或直接调用 HTTP API。
- 所有受保护 API 必须继续验证 Bearer access token。
- 权限相关接口必须继续在服务端读取当前权限状态。
- 前端收到 `401` 或 `403` 时只能提供正确反馈，不能自行授予权限。

## 实现文件

| 文件 | 已实现职责 |
| --- | --- |
| `frontend/src/api/auth.ts` | 增加 logout DTO 和 204 请求函数 |
| `frontend/src/auth/session.ts` | 增加 AuthStatus、单一初始化 Promise、logout 用例和 logging-out 状态 |
| `frontend/src/auth/coordination.ts` | 复用现有锁名串行执行 refresh/logout，并将公开函数命名扩展为会话锁 |
| `frontend/src/router/guards.ts` | 实现全局守卫、回跳校验和会话失效监听 |
| `frontend/src/router/index.ts` | 导出供 main 安装守卫的共享 Router |
| `frontend/src/main.ts` | 启动同步、安装守卫并提前触发统一初始化 |
| `frontend/src/layouts/DesktopShell.vue` | 增加退出按钮、加载状态、结果导航和 `unavailable` 连接状态提示 |
| `frontend/src/pages/login/DesktopLoginPage.vue` | 登录成功后恢复合法 redirect；显示本机退出警告 |
| `docs/code-map/frontend.md` | 记录模块职责和新增守卫文件 |
| `docs/frontend/api-client.md` | 记录登出、五态会话和多标签页语义 |
| `docs/frontend/routes.md` | 记录实际守卫和安全回跳规则 |

## 落地顺序

1. 扩展 `api/auth.ts` 的 logout 契约。
2. 给会话层增加显式 AuthStatus 和单一初始化 Promise。
3. 在会话层实现与 refresh 共锁的 logout 用例。
4. 新增 `router/guards.ts`，先完成受保护路由与初始化等待。
5. 完成登录后的安全回跳。
6. 增加桌面退出按钮和 `local_only` 提示。
7. 增加会话失效监听和多标签页跳转。
8. 更新代码地图与现状文档。
9. 执行自动检查和真实浏览器多标签页验收。

## 验收矩阵

| 场景 | 预期结果 |
| --- | --- |
| 未登录直接访问 `/dashboard` | 等待初始化后进入登录页，redirect 保存 `/dashboard` |
| 持久 token 有效时刷新 `/items` | 先恢复会话，再停留 `/items`，不闪跳登录页 |
| 持久 token 已失效 | 清除记录并进入登录页 |
| 恢复时服务离线 | 保留持久 token，不误显示“凭据失效”；页面显示连接异常状态 |
| 登录成功且存在合法 redirect | 使用 replace 返回目标页面 |
| redirect 为外部 URL | 拒绝并进入 dashboard |
| 已登录访问登录页 | 重定向 dashboard |
| 正常点击退出 | 服务端 logout 204，本地清理，进入登录页 |
| logout 返回 invalid_refresh_token | 视为已退出，本地清理，不显示错误 |
| logout 时网络失败 | 本地和其它标签页退出，提示服务端吊销未确认 |
| A 标签页退出，B 停留 dashboard | B 清除内存并自动进入登录页 |
| A refresh、B logout 同时发生 | 共用锁，最终没有持久 token，所有标签页退出 |
| 同标签页重复点击退出 | 只执行一个 logout Promise，按钮保持禁用 |
| 退出后浏览器后退 | 不重新进入已登录页面；守卫继续拦截 |
| 未登录直接请求受保护 API | 服务端仍返回 401，证明安全不依赖前端守卫 |

## 验证命令与浏览器检查

实现完成后至少执行：

```powershell
cd frontend
pnpm build
```

浏览器验收必须检查：

- 登录、刷新恢复和原目标回跳。
- 正常 logout 请求为 204。
- logout 后 localStorage 不再包含 `winestock.auth.session.v1`。
- access token 从未写入 localStorage 或 sessionStorage。
- 双标签页退出同步和 refresh/logout 并发。
- 服务离线时本地退出与提示行为。
- 控制台无未处理 Promise、重复导航或无限重定向错误。

## 2026-07-10 验收结果

- `frontend` 下执行 `pnpm build` 通过，Vue 类型检查和 Vite 生产构建均成功。
- 未登录访问 `/dashboard`、`/items` 会进入登录页并保留内部 redirect。
- 有效持久 token 刷新 `/items` 后停留原页面；失效 token 会被清除并进入登录页。
- 合法 redirect 登录后回到 `/items`；外部 URL redirect 被拒绝并进入 dashboard。
- 已登录访问登录页会回到 dashboard；正常登出网络请求返回 204，本地记录和内存会话均被清除。
- 服务请求失败时保留持久 token 和受保护页面，并显示“服务连接异常”；此状态下主动登出会清除本地记录并显示固定的吊销未确认提示。
- 双标签页中一页退出后，另一页清除内存会话并进入登录页；refresh/logout 并发后最终无持久 token。
- 同标签页重复登出返回同一个 Promise；服务端已吊销 token 时结果为 `already_invalid`，本地仍完成退出。
- logout 后浏览器后退不会重新进入受保护页面；未携带 access token 请求 `/api/auth/me` 仍返回 401。
- localStorage/sessionStorage 均未写入 access token；除验收中主动触发的预期 401 外，控制台没有未处理 Promise、重复导航或无限重定向错误。

## 完成标准

满足以下条件后，才把“登出 UI/API 和路由守卫”标记为完成：

- 服务端最新 refresh token 可以被前端 logout 用例可靠吊销。
- 本地清理和跨标签页退出始终执行。
- 路由守卫等待初始化，不以未完成恢复的空会话作判断。
- 明确匿名用户不能进入受保护页面。
- 网络不可用与凭据失效使用不同状态和提示。
- 登录回跳只接受内部路径。
- 前端守卫没有替代或削弱服务端鉴权。
- 构建、注释、代码地图和真实浏览器验收全部通过。
