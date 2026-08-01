# 代码地图

本文件是 WineStock 分层代码地图的总索引。
具体源码职责按项目结构拆分维护，不在本文件重复堆叠所有模块细节。
根 `docs/` 只保存跨组件规范和本代码地图；组件实现文档位于对应组件的 `docs/` 目录。

模块或领域的职责、边界或依赖方向变化，以及新增或移动模块、crate、页面域和共享组件域时，必须在同一改动中更新对应子地图。
新增顶层组件或改变依赖方向时，同时更新本索引。
所有代码地图必须使用中文编写和维护。

## 粒度约定

所有子地图遵守以下约定；超出预算视为粒度失控信号，应先合并条目而不是继续追加：

1. **条目单位是模块/目录/领域，不是文件。** 一个条目覆盖一组内聚文件，不逐文件各写一行；逐文件职责由源码中文文件头注释承担（见 `docs/agent-checklist.md` 的 Comment Check）。
2. **每个条目只记录三件事：** 拥有的职责；明确不拥有的边界；依赖方向或被谁使用。不复述文件内部行为细节、交互流程或验收规则——那些属于文件头注释和组件 `docs/`。
3. **仍可点名单个文件的例外情形：** 公共入口点（`lib.rs`、`main.ts`、bootstrap、路由装配）；跨组件契约（如 `frontend/src/shell/contract.ts`）；机制不寻常、无法从目录结构推断所有权的文件（如 `src/tests/` 的 `#[path]` 测试布局）。
4. **行数预算：** 每份子地图目标 ≤ 100 行（`core/http-api.md` 这类路径清单除外）。
5. **文档清单单一来源：** 组件 `docs/` 下的设计文档清单由组件自己的 `docs/README.md` 索引维护；地图只链接该索引一次，不复制文档列表。

## 子地图

- [`code-map/workspace.md`](code-map/workspace.md)：仓库根目录、当前范围、依赖方向、测试布局和验证入口。
- [`code-map/shared.md`](code-map/shared.md)：`shared` 平台无关配置与基础校验 crate。
- [`code-map/core.md`](code-map/core.md)：`core` Axum 服务、业务模块、持久化层和公共 HTTP 接口。
- [`code-map/server.md`](code-map/server.md)：`server` 无头平台 shell 和启动流程。
- [`code-map/frontend.md`](code-map/frontend.md)：`frontend` 路由、布局、API client、鉴权会话和页面。
- [`code-map/desktop.md`](code-map/desktop.md)：`desktop/tauri` Tauri 窗口、Shell Bridge、配置持久化和本地 core 生命周期。
- [`code-map/android.md`](code-map/android.md)：`android` 原生 shell、WebView 加载、Shell Bridge 传输和运行配置。

## 顶层所有权

```text
brand       -> 跨平台品牌矢量母版，不拥有平台资源格式
shared      -> 平台无关配置与基础规则
core        -> 共享 Axum 服务和业务能力
server      -> 无头服务端平台 shell
frontend    -> 共享前端源码，不由 Axum 服务
desktop     -> Tauri v2 桌面 shell、Windows 窗口/资源打包、Shell Bridge 与本地 core 生命周期
android     -> 原生 WebView shell，已实现 Shell Bridge、运行配置、edge-to-edge inset 与 Application 级本地 Axum
```

详细架构边界仍以 `docs/architecture.md`、`docs/platforms.md` 和 `docs/project-structure.md` 为准。
