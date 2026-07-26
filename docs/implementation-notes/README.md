# Implementation Notes

This directory stores cross-component implementation plans, design notes, and historical decision drafts.

Files here are non-normative by default. They do not add standing agent constraints unless a normative project document explicitly references them or a user asks to use one for a task.

Component-specific notes belong to `core/docs/implementation-notes/`, `frontend/docs/implementation-notes/` or the corresponding component directory.

## Current notes

- `json-config-and-db-auth-settings.md`：同时涉及 shared 配置边界与 core 数据库存储的历史方案。
- `item-catalog-inventory-monitoring.md`：物品目录实时库存聚合、补货筛选、固定列表格、移动库存项目和多页物品 Dialog 的跨 core/frontend 完整实施方案。
- `item-catalog-structured-filters.md`：物品目录分类、模板、单位、库位和可搜索模板属性的结构化筛选契约、前端面板与验收方案。
- `unified-item-attribute-definitions.md`：统一物品模板属性与物品自定义属性定义实体的跨 core/frontend 实施方案、确认决策和测试数据库转换步骤。
- `category-template-item-usage-counts.md`：分类与物品属性模板的物品使用数量、删除影响提示和跨 core/frontend 验收整改方案。
- `android-webview-edge-to-edge.md`：Android WebView 全屏铺设、原生 WindowInsets 向前端安全区变量传递、系统栏外观与跨 Android/frontend 验收实施方案。
- `android-frontend-packaging-workflow.md`：Android 自动构建、校验、暂存并验证共享前端资源的跨 Android/frontend 打包工作流方案。
- `android-webview-native-back-navigation.md`：Android 原生返回键通过 Shell Bridge 先交给前端处理 Dialog、Drawer、页面步骤与路由，并在未处理或超时后安全回退的跨 Android/frontend 实施方案。
- `server-mode-lan-access-discovery.md`：server mode 当前设备真实局域网地址的 Shell 所有权、本机运行设置与头像双入口、地址清理、响应式 Dialog 和分阶段验收方案。
- `android-embedded-core-integration.md`：Android 通过独立 JNI Rust 适配层引入共享 core、建立应用级本地 Axum 生命周期、配置事务、存储、ABI 打包与验收的跨 shared/core/server/android 详细实施方案。
- `self-hosted-auto-port.md`：本机模式隐藏端口、由 Shell 自动分配并持久化实际端口的跨 shared/core/android/frontend 实施方案。
- `openapi-frontend-typegen.md`：core Debug OpenAPI 离线导出、前端 openapi-typescript 类型生成、别名桥接与漂移防线的跨 core/frontend 实施方案。
- `code-map-granularity-reduction.md`：代码地图从逐文件镜像收敛为模块/职责粒度的约定、逐地图整改与流程规则同步方案。
- `inbound-template-removal.md`：删除入库模板概念的跨 core/frontend 影响评估、收货属性去留的方案取舍与实施决策记录（已按方案 B 实施）。
