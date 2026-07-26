# Agent Instructions

本文件是 WineStock 全项目的短操作入口。
跨组件设计放在根 `docs/`，组件实现规则放在所属组件的 `docs/`；不要把某个领域的局部约束当成整个项目的默认架构。

## 开始工作前

所有代码或结构修改先读取：

1. [`docs/README.md`](docs/README.md)
2. [`docs/architecture.md`](docs/architecture.md)
3. [`docs/project-structure.md`](docs/project-structure.md)
4. [`docs/agent-checklist.md`](docs/agent-checklist.md)
5. [`docs/code-map.md`](docs/code-map.md) 及本次改动对应的子地图

涉及运行模式、地址、端口或服务生命周期时，再读取：

- [`docs/runtime-networking.md`](docs/runtime-networking.md)
- [`docs/platforms.md`](docs/platforms.md)

进入具体领域时，从对应文档入口继续：

- [`core/docs/README.md`](core/docs/README.md)：Axum HTTP、业务逻辑、持久化、权限和数据库。
- [`shared/docs/README.md`](shared/docs/README.md)：平台无关配置、配置加载和基础规则。
- [`server/docs/README.md`](server/docs/README.md)：无头 server shell、固定配置位置、启动与关闭。
- [`frontend/docs/README.md`](frontend/docs/README.md)：前端路由、API client、页面、交互、视觉和响应式规则。

正式 desktop Tauri shell 尚未实现；Android shell 已实现 Application 级本地 core/JNI 与 APK 打包，但真实设备验收仍待完成。平台工作必须先依据 [`docs/platforms.md`](docs/platforms.md) 和 [`docs/project-structure.md`](docs/project-structure.md) 明确所有权，不能复制 demo 或脚手架结构作为正式架构。

## 项目所有权

- `shared`：平台无关配置与基础规则，不依赖 Axum、平台 shell 或前端。
- `core`：共享 Rust/Axum 服务、HTTP 契约、业务逻辑、状态和持久化，不拥有平台 UI 或资源打包。
- `server`：无头进程生命周期、配置位置、日志、启动和优雅关闭，不复制 core 业务实现。
- `frontend`：共享 Vue/Vite 前端源码，通过 HTTP 使用 core，不直接调用 Rust 内部业务 API。
- `desktop`、`android`：各自的平台生命周期、WebView、权限和资源打包；正式实现必须复用同一 core 服务。

允许的主要依赖方向：

```text
server -> core -> shared
server -> shared
frontend -> HTTP API
desktop/android shell -> core/shared + packaged frontend assets
```

禁止 core 依赖平台 shell 或前端构建产物，也禁止 Axum 打包或服务平台前端资源。

## 实施规则

- 先确认变更归属和公共边界，再修改代码；跨组件变更要明确每一侧的职责。
- 未明确指定交付格式时，评估、分析、设计和实施方案等报告默认生成 Markdown（`.md`）并放入所属 `docs/`；只有用户明确要求 Word/DOCX、PDF 或其它格式时才生成对应文件，不得仅因“报告”或“正式文档”等表述自行改用二进制文档格式。
- 优先沿用现有模块、组件、token、API 契约和局部模式，不建立重复实现。
- 引入依赖、修改数据库、删除兼容行为、更新代码地图、编写中文注释和选择验证范围时，完整执行 [`docs/agent-checklist.md`](docs/agent-checklist.md)。
- 判断 HTTP 接口时先核对运行服务的 `/api-docs/openapi.json`，再读 core 业务文档，最后才追踪 controller/service/repository 源码。
- UI 改动必须依次读取前端视觉规范、`frontend/docs/ui-design-guidelines.md`、一致性清单和异步动效文档，并在桌面、断点附近和移动视口检查真实尺寸、溢出、状态与控制台。
- 普通查询列表的服务端分页只作为数据获取协议，界面统一采用列表尾部哨兵触底加载；不得新增“上一页/下一页”翻页器。确有例外时，必须在对应页面文档记录业务原因、替代交互和验收结果。
- 不硬编码 IP 或端口；`0.0.0.0` 只能用于绑定，不能作为浏览器或 WebView 访问 URL。
- 删除或改变行为时同步清理失效代码、测试、注释、文档和代码地图，不保留无意义兼容层。

## 完成门槛

- 使用覆盖本次变更的最窄有效检查；跨 crate、公共 API、依赖或发布检查才扩大验证范围。
- 审核变更源码中的中文注释、所有权说明和过时内容。
- 确认工作区现有用户改动未被回退，差异只包含本任务需要的内容。
- 模块或领域的职责、边界或依赖方向变化必须同步更新相应中文代码地图；地图粒度遵守 `docs/code-map.md` 的粒度约定，逐文件细节由源码文件头注释承担。
- 最终说明实际执行的构建、测试、浏览器或 smoke 检查，以及未能执行的验证。
