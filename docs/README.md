# 项目级文档

本目录只保存 WineStock 跨组件的架构、平台边界、运行网络、Shell Bridge、项目结构、代理约束和全仓库代码地图。
具体实现文档由对应组件目录负责，避免把 `core`、`shared`、`server` 或前端的局部规则误认为整个项目的统一约束。

## 项目级规范

- [`architecture.md`](architecture.md)：整体组件与依赖边界。
- [`runtime-networking.md`](runtime-networking.md)：运行模式、绑定地址和访问 URL 规则。
- [`platforms.md`](platforms.md)：desktop、Android、server 和前端平台职责。
- [`shell-bridge.md`](shell-bridge.md)：UI 平台的前端运行配置、Shell Bridge、服务生命周期和错误边界。
- [`project-structure.md`](project-structure.md)：仓库结构和组件所有权。
- [`agent-checklist.md`](agent-checklist.md)：实施与验证检查清单。
- [`code-map.md`](code-map.md)：全仓库代码地图入口。

## 组件文档

- [`../core/docs/README.md`](../core/docs/README.md)：Axum 服务、业务 API、数据库、权限和 core 实现说明。
- [`../shared/docs/README.md`](../shared/docs/README.md)：共享配置、配置文件加载和基础校验说明。
- [`../server/docs/README.md`](../server/docs/README.md)：无头 server shell 的配置与生命周期说明入口。
- [`../frontend/docs/README.md`](../frontend/docs/README.md)：前端页面、路由、API client、交互和视觉规则。

`docs/implementation-notes/` 只保留确实跨组件的历史方案或决策草稿。单一组件的实现记录应放在该组件自己的 `docs/implementation-notes/` 下。
