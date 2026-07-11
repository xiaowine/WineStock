# 前端路由

本文记录 `frontend/` 当前已实现的路由、history 策略和布局边界。
路由存在不代表页面内容、导航排序或业务交互已经定稿。

## History 策略

当前使用 Vue Router 的 `createWebHashHistory`。

选择 hash history 的原因：

- 共享前端最终由 Desktop Tauri 和 Android 平台 shell 分别打包。
- 打包资源不能依赖 Axum 或其它 HTTP 服务提供 SPA fallback。
- Axum 不拥有也不服务前端构建产物。
- hash 中的客户端路径不会改变平台请求的资源文件路径，更适合尚未确定的 WebView 资源协议。

开发环境中的页面地址形如：

```text
http://127.0.0.1:<vite-port>/#/dashboard
http://127.0.0.1:<vite-port>/#/items
http://127.0.0.1:<vite-port>/#/inbound
http://127.0.0.1:<vite-port>/#/users
```

如果后续平台 shell 明确提供可靠的 history fallback，可以重新评估是否改用 `createWebHistory`。

## 当前路由

| 路径 | 路由名 | 布局 | `requiresAuth` | 当前职责 |
| --- | --- | --- | --- | --- |
| `/` | 无 | `AppShell` | 是 | 重定向到 `dashboard` |
| `/dashboard` | `dashboard` | `AppShell` | 是 | 库存摘要、出入库趋势和呆滞物品总览；需要 `stock.dashboard.read` |
| `/items` | `items` | `AppShell` | 是 | 物品列表、新建/编辑、分类、可选属性预设和自定义属性 |
| `/inbound` | `inbound` | `AppShell` | 是 | 桌面端多明细入库工作台，支持本地草稿恢复、同物品多批次和模板图片；创建 pending 单据后留在当前页；需要 `stock.inbound.create`，移动导航暂不展示入口 |
| `/users` | `users` | `AppShell` | 是 | 用户管理真实列表和管理操作；另需 `user.read` |
| `/login` | `login` | 独立响应式页面 | 否 | 桌面和移动共用真实登录表单 |
| `/register` | `register` | 独立响应式页面 | 否 | 桌面和移动共用首个用户注册及自动登录流程 |
| `/change-password` | `change-password` | 独立响应式页面 | 是 | 当前用户主动改密；强制改密用户唯一允许进入的前端页面 |
| `/:pathMatch(.*)*` | `home-fallback` | 无独立页面 | 否 | 未匹配路径直接重定向到 `dashboard` |

## 布局和页面边界

- `App.vue` 只提供根 `RouterView`。
- `AppShell.vue` 根据视口断点选择 `DesktopShell` 或 `MobileShell`。
- `DesktopShell.vue` 和 `MobileShell.vue` 各自提供嵌套 `RouterView`，页面组件本身保持共享。
- 一级导航配置集中在 `router/navigation.ts`；用户入口按当前会话的 `user.read` 权限显示。
- 登录、注册和修改密码页面不进入业务应用壳；未匹配路径不渲染独立页面，直接返回总览。

## 鉴权状态

`frontend/src/router/guards.ts` 已经统一执行 `requiresAuth`，业务页面不再分散判断登录状态：

- 路由声明 `requiredPermission` 时，缺少对应权限的已登录用户会返回总览。
- 该判断只用于前端导航，Axum 仍按数据库当前权限执行最终授权。

- 初始导航等待会话层的同一个初始化 Promise，避免恢复完成前闪跳登录页。
- 只有明确 `anonymous` 才把受保护页面 replace 到 `/login?redirect=<内部路径>`。
- `unavailable` 保留目标页面和持久 token，由应用壳显示服务连接异常。
- 已登录访问登录页时进入 dashboard。
- 登录或 refresh 返回 `password_change_required = true` 时，守卫只允许进入修改密码页；原受保护目标保存在 `redirect` query 中。
- 停留期间恢复出强制改密状态时，监听器也会离开当前业务页面并进入修改密码页。
- 修改密码成功后清除当前会话用户摘要中的强制改密标记，并恢复合法内部目标或进入 dashboard。
- 登录成功后只接受以 `/` 开头、非 `//`、无反斜杠且能匹配现有路由的内部 redirect；其它值进入 dashboard。
- 会话在停留期间变为 `anonymous` 时，监听器会把当前受保护页面 replace 到登录页。
- 一个标签页退出后，其它标签页通过 storage 事件清理内存会话并触发相同导航。

完整状态模型、登出竞态和验收记录见 [`auth-logout-and-route-guards.md`](auth-logout-and-route-guards.md)。

## 尚未确认

- 除总览和物品外的一级模块路由与排序。
- 路由和具体权限代码的完整映射。
- 用户权限变化后的导航可见性与接口 `403` 状态。
- 页面参数、详情页层级、编辑页是独立页面还是面板。
- Android 返回键和平台深链接行为。
