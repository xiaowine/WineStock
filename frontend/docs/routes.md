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
http://127.0.0.1:<vite-port>/#/inbound/orders
http://127.0.0.1:<vite-port>/#/outbound
http://127.0.0.1:<vite-port>/#/outbound/orders
http://127.0.0.1:<vite-port>/#/approvals/inbound
http://127.0.0.1:<vite-port>/#/approvals/outbound
http://127.0.0.1:<vite-port>/#/locations
http://127.0.0.1:<vite-port>/#/templates
http://127.0.0.1:<vite-port>/#/substitutes
http://127.0.0.1:<vite-port>/#/events
http://127.0.0.1:<vite-port>/#/users
```

如果后续平台 shell 明确提供可靠的 history fallback，可以重新评估是否改用 `createWebHistory`。

## 当前路由

应用壳一级页面的名称、权限和导航呈现集中声明在 `src/router/appRouteCatalog.ts`。路由 `meta`、桌面侧栏、移动 Drawer、移动 Header 和页面主标题均读取同一份元数据，不得在组件或导航配置中再次硬编码页面名称。

| 路径                  | 路由名               | 布局           | `requiresAuth` | 当前职责                                                                                                                                                                                |
| --------------------- | -------------------- | -------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/`                   | 无                   | `AppShell`     | 是             | 重定向到 `dashboard`                                                                                                                                                                    |
| `/dashboard`          | `dashboard`          | `AppShell`     | 是             | 库存摘要、出入库趋势和呆滞物品总览；需要 `stock.dashboard.read`                                                                                                                         |
| `/items`              | `items`              | `AppShell`     | 是             | 全宽物品目录，以及复用通用 Dialog 的新建/编辑、分类、可选属性预设和自定义属性；需要 `stock.item.read`                                                                                   |
| `/inbound`            | `inbound`            | `AppShell`     | 是             | 桌面和移动端多明细入库工作台；提交按钮按权限自动切换为“直接入库”或“提交审核”，不允许同时选择两种模式；拥有物品管理权限时可在选择阶段新建物品并直接加入草稿；需要 `stock.inbound.create` |
| `/inbound/orders`     | `inbound-orders`     | `AppShell`     | 是             | 真实入库单服务端分页、关键词/状态/日期筛选和按需详情；需要 `stock.inbound.read`，审批入口仍需要 `stock.inbound.approve`                                                                 |
| `/outbound`           | `outbound`           | `AppShell`     | 是             | 两步新建待审批出库工作台，支持物品搜索、FIFO/指定批次、草稿恢复和提交确认；创建需要 `stock.outbound.create`，选品与批次读取还需要 `stock.item.read`，移动导航暂不展示入口               |
| `/outbound/orders`    | `outbound-orders`    | `AppShell`     | 是             | 真实出库单服务端分页协议下的触底追加、关键词/状态/日期筛选和按需详情；需要 `stock.outbound.read`，审批入口仍需要 `stock.outbound.approve`                                               |
| `/approvals/inbound`  | `inbound-approvals`  | `AppShell`     | 是             | 待审批入库单队列、关键词/日期筛选、按需详情、确认通过与拒绝；同时需要 `stock.inbound.read` 和 `stock.inbound.approve`                                                                   |
| `/approvals/outbound` | `outbound-approvals` | `AppShell`     | 是             | 待审批出库单队列、关键词/日期筛选、批次/FIFO 详情、确认通过与拒绝；同时需要 `stock.outbound.read` 和 `stock.outbound.approve`                                                           |
| `/locations`          | `locations`          | `AppShell`     | 是             | 真实库位分组树、库位搜索和分组/库位 CRUD；读取需要 `stock.location.read`，管理操作需要 `stock.location.manage`；整批次移库等待按库位查询批次契约                                        |
| `/templates`          | `templates`          | `AppShell`     | 是             | 真实物品分类、物品属性模板和入库模板列表、查看、创建、编辑、复制与差异化删除确认；需要 `stock.template.read`，写操作需要 `stock.template.manage`                                        |
| `/substitutes`        | `substitutes`        | `AppShell`     | 是             | 全局替代关系治理页面；需要 `stock.substitute.read`，编辑需要 `stock.substitute.manage`                                                                                                  |
| `/events`             | `events`             | `AppShell`     | 是             | 真实审计日志筛选、自动加载、三段式列表和历史 JSON 详情；需要 `audit.read`                                                                                                               |
| `/users`              | `users`              | `AppShell`     | 是             | 用户管理真实列表和管理操作；另需 `user.read`                                                                                                                                            |
| `/login`              | `login`              | 独立响应式页面 | 否             | 桌面和移动共用真实登录表单                                                                                                                                                              |
| `/register`           | `register`           | 独立响应式页面 | 否             | 桌面和移动共用首个用户注册及自动登录流程                                                                                                                                                |
| `/change-password`    | `change-password`    | 独立响应式页面 | 是             | 当前用户主动改密；强制改密用户唯一允许进入的前端页面                                                                                                                                    |
| `/:pathMatch(.*)*`    | `home-fallback`      | 无独立页面     | 否             | 未匹配路径直接重定向到 `dashboard`                                                                                                                                                      |

## 布局和页面边界

- `App.vue` 只提供根 `RouterView`。
- `AppShell.vue` 始终保持同一棵应用框架 DOM，通过 CSS 在桌面和移动端重排，并只提供一个嵌套 `RouterView`。
- 移动导航 Drawer 只覆盖导航节点，不包含也不销毁当前路由页面；页面组件和业务状态在断点变化时保持不变。
- 一级导航配置集中在 `router/navigation.ts`；OpenAPI 业务域路由已补充对应侧栏入口，并继续复用现有业务/管理分组和图标样式。
- 入库审批与出库审批路由使用同一个库存审批工作台，只由两个薄路由页注入领域差异；历史单据查询仍留在入库单、出库单页面。
- 登录、注册和修改密码页面不进入业务应用壳；未匹配路径不渲染独立页面，直接返回总览。

## 鉴权状态

`frontend/src/router/guards.ts` 已经统一执行 `requiresAuth`，业务页面不再分散判断登录状态：

- 普通路由声明 `requiredPermission`；需要组合能力的路由声明 `requiredPermissions`，任一权限缺失时返回当前会话可访问的默认页面。
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

- 用户权限变化后的导航可见性与接口 `403` 状态。
- 页面参数、详情页层级、编辑页是独立页面还是面板。
- Android 返回键和平台深链接行为。
