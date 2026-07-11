# Core 文档

本目录记录 `winestock-core` 拥有的 Axum HTTP 契约、业务规则、数据库结构、权限模型、字段校验和历史实现方案。
这些文档描述 core 服务能力，不代表平台 shell、前端界面或整个仓库的统一实现方式。

## 主要入口

- [`business-api.md`](business-api.md)：业务 API 总览。
- [`business-api/`](business-api/)：按业务域拆分的接口契约。
- [`database-schema.md`](database-schema.md)：数据库表、字段和索引边界。
- [`rbac-permission-model.md`](rbac-permission-model.md)：权限模型。
- [`user-management-api.md`](user-management-api.md)：用户管理 API。
- [`validation/`](validation/)：core DTO、entity 和 repository 校验说明。
- [`implementation-notes/`](implementation-notes/)：core 历史方案和非规范性实现记录。

跨组件架构和运行约束仍以仓库根目录 [`../../docs/README.md`](../../docs/README.md) 中列出的项目级文档为准。
