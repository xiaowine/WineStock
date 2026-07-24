# 代码地图

本文件是 WineStock 分层代码地图的总索引。
具体源码职责按项目结构拆分维护，不在本文件重复堆叠所有模块细节。
根 `docs/` 只保存跨组件规范和本代码地图；组件实现文档位于对应组件的 `docs/` 目录。

生成新代码、增加或移动模块和 crate、修改公共 API，或进行较大范围实现改动后，必须更新对应子地图。
新增顶层组件或改变依赖方向时，同时更新本索引。
所有代码地图必须使用中文编写和维护。

## 子地图

- [`code-map/workspace.md`](code-map/workspace.md)：仓库根目录、当前范围、依赖方向、测试布局和验证入口。
- [`code-map/shared.md`](code-map/shared.md)：`shared` 平台无关配置与基础校验 crate。
- [`code-map/core.md`](code-map/core.md)：`core` Axum 服务、业务模块、持久化层和公共 HTTP 接口。
- [`code-map/server.md`](code-map/server.md)：`server` 无头平台 shell 和启动流程。
- [`code-map/frontend.md`](code-map/frontend.md)：`frontend` 路由、布局、API client、鉴权会话和页面。
- [`code-map/android.md`](code-map/android.md)：`android` 原生 shell、WebView 加载、Shell Bridge 传输和运行配置。

## 顶层所有权

```text
shared      -> 平台无关配置与基础规则
core        -> 共享 Axum 服务和业务能力
server      -> 无头服务端平台 shell
frontend    -> 共享前端源码，不由 Axum 服务
desktop     -> 当前非正式脚手架，正式 Tauri shell 尚未实现
android     -> 原生 WebView shell，已实现 Shell Bridge、运行配置、edge-to-edge inset 与 Application 级本地 Axum
```

详细架构边界仍以 `docs/architecture.md`、`docs/platforms.md` 和 `docs/project-structure.md` 为准。
