# Implementation Notes

This directory stores cross-component implementation plans, design notes, and historical decision drafts.

Files here are non-normative by default. They do not add standing agent constraints unless a normative project document explicitly references them or a user asks to use one for a task.

Component-specific notes belong to `core/docs/implementation-notes/`, `frontend/docs/implementation-notes/` or the corresponding component directory.

## Current notes

- `json-config-and-db-auth-settings.md`：同时涉及 shared 配置边界与 core 数据库存储的历史方案。
- `item-catalog-inventory-monitoring.md`：物品目录实时库存聚合、补货筛选、固定列表格、移动库存项目和多页物品 Dialog 的跨 core/frontend 完整实施方案。
- `item-catalog-structured-filters.md`：物品目录分类、模板、单位、库位和可搜索模板属性的结构化筛选契约、前端面板与验收方案。
- `unified-item-attribute-definitions.md`：统一物品模板属性与物品自定义属性定义实体的跨 core/frontend 实施方案、确认决策和测试数据库转换步骤。
