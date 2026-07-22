# 前端文档

本目录记录 `frontend/` 共享前端源码的页面框架、样式规则、页面清单、组件约定和后续实施说明。
当前已经完成页面框架、响应式 Shell、API client、统一会话恢复、注册/登录/修改密码、登出、鉴权与强制改密守卫、用户管理、库存总览、物品管理、库位主数据管理、分类与模板、替代关系全局治理、审计日志和正式入库草稿工作台；其它库存业务页面按路由与 API 实现状态继续推进。

前端文档只约束前端应用源码和 WebView 内的 UI 行为。
Desktop Tauri shell、Android native shell、WebView 生命周期、平台权限和资源打包仍由各自平台目录负责。
Axum 不服务前端构建产物。

UI 平台的首次设置、API 地址、运行配置和服务恢复界面由本前端统一拥有；配置持久化与服务启停通过根文档 [`../../docs/shell-bridge.md`](../../docs/shell-bridge.md) 定义的 Shell Bridge 交给平台壳执行。
业务能力继续通过 HTTP 调用 core，Shell Bridge 不代理业务 API。

## 当前文档

- `page-framework.md`：当前认可的页面框架、桌面/移动布局、导航职责和样式层级；不确认具体业务页面内容。
- `page-locations.md`：库位管理页面的职责边界、分组树、库位 CRUD、库存位置查看、整批次移库和实施验收。
- `page-events.md`：审计日志页面的服务端筛选、三段式列表、历史 JSON 详情、分页、响应式和实施验收设计。
- `page-inbound-orders.md`：入库单列表、服务端筛选、按需详情、审批路由边界、响应式和实施验收设计。
- `page-outbound-orders.md`：出库单列表、服务端筛选、触底追加、批次/FIFO 语义、按需详情和审批边界实施设计。
- `page-stock-approvals.md`：入库审批与出库审批共用工作台、审核详情、库存影响、权限组合、并发错误和响应式实施设计。
- `page-outbound.md`：新建出库两步工作台、批次/FIFO 分配、草稿、提交审核、响应式和验收设计。
- `page-templates.md`：分类、物品属性模板与入库模板页面的信息架构、字段编辑、危险操作、响应式和实施验收设计。
- `page-substitutes.md`：替代关系全局治理页面的关系分组、物品 Dialog 复用、整体保存、权限、响应式和实施验收设计。
- `page-runtime-settings.md`：无 API 依赖的运行设置、默认端口 `17890`、模式切换、Shell Bridge 边界和恢复入口。
- `routes.md`：当前路由、history 策略、嵌套 Shell 边界和鉴权元数据状态。
- `api-client.md`：运行时 API 地址、统一请求、错误契约、内存会话和当前注册/登录接入状态。
- `auth-logout-and-route-guards.md`：已实现的真正登出、会话初始化、路由守卫、多标签页退出和验收记录。
- `user-management.md`：用户列表、创建、启停、权限、临时密码和前后端授权边界。
- `visual-style.md`：当前视觉风格方向、颜色/圆角/阴影/密度规则和避免事项。
- `ui-design-guidelines.md`：后续业务页面必须遵守的页面骨架、三段式列表、工具栏、表单、浮层、响应式和例外规则。
- `ui-consistency-checklist.md`：按业务状态、目标视口和真实尺寸验证 UI 规范是否落实的实施与量化验收清单。
- `async-state-transitions.md`：加载、恢复、刷新和错误切换的防闪烁状态规则。
- `mobile-interactions.md`：浏览器/Android WebView 安全区变量、full-bleed 背景、固定操作区和移动端验收规则。
- `implementation-notes/`：只属于前端的历史方案和非规范性实施记录。
- `implementation-notes/inbound-template-usability-remediation.md`：入库模板可发现性、权限耦合、主列表状态、模板切换保护和分阶段整改方案。
- `implementation-notes/outbound-estimated-cost.md`：新建出库提交前的批次成本预估、FIFO 分摊、界面呈现、错误边界与验收方案。
- `implementation-notes/inbound-orders-mobile-remediation.md`：入库单列表移动端横向裁切的原因、单列条目重构、Dialog 适配与验收方案。
- `implementation-notes/substitute-network-visualization.md`：替代关系星链网络的入口、力导向布局、交互、响应式、性能和分阶段实施方案。
- `implementation-notes/runtime-tabs-consistency-remediation.md`：运行设置模式切换与全站 tab / 工作区导航 / 分段控件不一致的盘点与分阶段整改方案。

## UI 文档使用顺序

所有新增或重构 UI 依次读取：

1. `visual-style.md`：确认视觉语言和 token。
2. `ui-design-guidelines.md`：选择已有页面与交互模式。
3. `ui-consistency-checklist.md`：确定需要覆盖的状态和量化验收。
4. `async-state-transitions.md`：涉及异步变化时确认稳定呈现边界。

页面专属文档只补充业务字段、权限和流程，不得静默建立另一套通用视觉规则。

## 后续建议拆分

- `components.md`：按钮、输入框、表格、列表、抽屉、Dialog 等通用组件约定。
- `page-inventory.md`、`page-users.md` 等：复杂业务页面的内容结构、字段、操作和交互记录。
