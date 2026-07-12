# 前端文档

本目录记录 `frontend/` 共享前端源码的页面框架、样式规则、页面清单、组件约定和后续实施说明。
当前已经完成页面框架、响应式 Shell、API client、统一会话恢复、注册/登录/修改密码、登出、鉴权与强制改密守卫、用户管理、库存总览、物品管理和正式入库草稿工作台；其它库存业务页面按路由与 API 实现状态继续推进。

前端文档只约束前端应用源码和 WebView 内的 UI 行为。
Desktop Tauri shell、Android native shell、WebView 生命周期、平台权限和资源打包仍由各自平台目录负责。
Axum 不服务前端构建产物。

## 当前文档

- `page-framework.md`：当前认可的页面框架、桌面/移动布局、导航职责和样式层级；不确认具体业务页面内容。
- `routes.md`：当前路由、history 策略、嵌套 Shell 边界和鉴权元数据状态。
- `api-client.md`：运行时 API 地址、统一请求、错误契约、内存会话和当前注册/登录接入状态。
- `auth-logout-and-route-guards.md`：已实现的真正登出、会话初始化、路由守卫、多标签页退出和验收记录。
- `user-management.md`：用户列表、创建、启停、权限、临时密码和前后端授权边界。
- `visual-style.md`：当前视觉风格方向、颜色/圆角/阴影/密度规则和避免事项。
- `ui-consistency-checklist.md`：多步骤流程、表单、列表、表格、抽屉、Dialog 和响应式页面的一致性实现与量化验收清单。
- `async-state-transitions.md`：加载、恢复、刷新和错误切换的防闪烁状态规则。
- `implementation-notes/`：只属于前端的历史方案和非规范性实施记录。

## 后续建议拆分

- `components.md`：按钮、输入框、表格、列表、抽屉、Dialog 等通用组件约定。
- `mobile-interactions.md`：Android/WebView 触控、安全区、返回键和移动端页面操作规则。
- `page-inventory.md`、`page-users.md` 等：复杂业务页面的内容结构、字段、操作和交互记录。
