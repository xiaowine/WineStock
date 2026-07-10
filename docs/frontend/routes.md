# 前端路由

本文记录 `frontend/` 当前已实现的路由骨架、history 策略和布局边界。
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
```

如果后续平台 shell 明确提供可靠的 history fallback，可以重新评估是否改用 `createWebHistory`。

## 当前路由

| 路径 | 路由名 | 布局 | `requiresAuth` | 当前职责 |
| --- | --- | --- | --- | --- |
| `/` | 无 | `AppShell` | 是 | 重定向到 `dashboard` |
| `/dashboard` | `dashboard` | `AppShell` | 是 | 总览页面骨架 |
| `/items` | `items` | `AppShell` | 是 | 物品列表页面骨架 |
| `/login` | `login` | 独立页面 | 否 | 桌面真实登录表单；移动端占位入口 |
| `/register` | `register` | 独立页面 | 否 | 桌面首个用户注册并自动登录；移动端仅说明 |
| `/:pathMatch(.*)*` | `not-found` | 独立页面 | 否 | 未匹配路径兜底 |

## 布局和页面边界

- `App.vue` 只提供根 `RouterView`。
- `AppShell.vue` 根据视口断点选择 `DesktopShell` 或 `MobileShell`。
- `DesktopShell.vue` 和 `MobileShell.vue` 各自提供嵌套 `RouterView`，页面组件本身保持共享。
- 一级导航配置集中在 `router/navigation.ts`，当前只包含总览和物品入口。
- 登录、注册和 404 页面不进入业务应用壳。

## 鉴权状态

`requiresAuth` 当前只是路由契约元数据，尚未执行登录拦截。
API client、localStorage 持久会话、启动恢复和 refresh 轮换已经建立，但路由守卫尚未实现。
在守卫完成前，不在各业务页面分散添加登录判断。

后续鉴权守卫需要统一处理：

- 未登录访问受保护页面时导航到登录页。
- 登录成功后恢复原目标页面。
- 未恢复出有效会话时统一进入登录页。
- 用户权限变化后的导航可见性与接口 `403` 状态。

## 尚未确认

- 除总览和物品外的一级模块路由与排序。
- 路由和具体权限代码的完整映射。
- 页面参数、详情页层级、编辑页是独立页面还是面板。
- 服务端 `password_change_required` 状态的前端呈现方式；当前没有独立修改密码页面。
- Android 返回键和平台深链接行为。
