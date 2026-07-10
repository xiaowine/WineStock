# 前端文档

本目录记录 `frontend/` 共享前端源码的页面框架、样式规则、页面清单、组件约定和后续实施说明。
当前已经确认页面框架、响应式 Shell、首批路由骨架、API client、统一 localStorage 会话恢复，以及桌面注册和登录表单；其它业务页面内容、字段、按钮、指标、筛选项和流程尚未确认。

前端文档只约束前端应用源码和 WebView 内的 UI 行为。
Desktop Tauri shell、Android native shell、WebView 生命周期、平台权限和资源打包仍由各自平台目录负责。
Axum 不服务前端构建产物。

## 当前文档

- `page-framework.md`：当前认可的页面框架、桌面/移动布局、导航职责和样式层级；不确认具体业务页面内容。
- `routes.md`：当前路由、history 策略、嵌套 Shell 边界和鉴权元数据状态。
- `api-client.md`：运行时 API 地址、统一请求、错误契约、内存会话和当前注册/登录接入状态。
- `visual-style.md`：当前视觉风格方向、颜色/圆角/阴影/密度规则和避免事项。

## 后续建议拆分

- `components.md`：按钮、输入框、表格、列表、抽屉、弹窗等通用组件约定。
- `mobile-interactions.md`：Android/WebView 触控、安全区、返回键和移动端页面操作规则。
- `page-inventory.md`、`page-users.md` 等：复杂业务页面的内容结构、字段、操作和交互记录。
