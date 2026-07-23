# Agent Checklist

Use this checklist before making implementation changes.

## Before Editing

Read:

- `docs/architecture.md`
- `docs/runtime-networking.md`
- `docs/platforms.md`
- `docs/shell-bridge.md`（涉及 UI 平台配置、Shell 生命周期、WebView 或前端启动恢复时）
- `docs/project-structure.md`
- `docs/code-map.md`
- 与当前改动对应的 `docs/code-map/*.md` 或 `docs/code-map/core/*.md` 子地图

Then identify:

- which component owns the change
- whether the change is shared or platform-specific
- whether the change affects runtime networking
- whether the change affects platform lifecycle
- whether frontend packaging is involved

Do not start by copying demo structure.

同时读取当前组件自己的文档入口：

- `core/docs/README.md`
- `shared/docs/README.md`
- `server/docs/README.md`
- `frontend/docs/README.md`

只需要读取与本次改动有关的组件入口和详细文档，不要求每次把所有领域文档全部展开。

## Ownership Check

If the change is API, business logic, service state, or bind behavior, it belongs in the `core axum library`.

If the change is window, activity, WebView, permissions, packaging, or OS lifecycle, it belongs in the platform shell.

If the change is headless process startup, service-only lifecycle, or deployment logging without UI, it belongs in the `server shell`.

If the change is UI rendering or frontend assets, it belongs in the frontend app or platform asset packaging.

If the change spans multiple parts, define the boundary first.

## Networking Check

Before changing a URL, bind address, or startup mode, confirm:

- runtime mode
- `bind_host`
- `port`
- `remote_base_url`
- `auto_start_server`

Use `127.0.0.1:<port>` for local self access.
Use actual LAN IP addresses for external access.
Never use `0.0.0.0` as an access URL.
Use `remote_base_url` when a platform connects to another service; a pure server shell may not need it for API-only hosting.

## Platform Lifecycle Check

Before changing server startup or shutdown, confirm:

- who starts Axum
- when Axum starts
- who stops Axum
- what happens on app background
- what happens on app exit
- how startup errors are reported
- how port conflicts are reported

Desktop, Android, and the server shell may have different lifecycle policies.
They should still call the same shared service library.

UI-bearing平台还必须确认：

- 前端资源在 API 未配置、未启动或启动失败时仍可加载。
- 运行配置和服务错误只由共享前端呈现，不新增原生设置 UI。
- 业务 API 仍走 HTTP，Shell Bridge 不代理业务或携带鉴权 token。
- 本地 `bind_host` 与前端实际 `apiBaseUrl` 分离，不能把 `0.0.0.0` 交给 WebView 访问。
- 修改 API 地址后会停止旧请求、重置会话边界并重新初始化健康检查。

## Frontend Packaging Check

Before adding or moving frontend files, confirm:

- which platform packages the assets
- whether the assets are platform-specific or shared source
- that build output is not placed in the Axum crate
- that Axum is not serving platform UI bundles

The frontend framework is not fixed by this spec.
Do not introduce one without user approval.

前端页面、交互、视觉和响应式实现还必须读取 `frontend/docs/README.md` 指向的相关规范；视觉语言以 `frontend/docs/visual-style.md` 为准，页面和交互模式以 `frontend/docs/ui-design-guidelines.md` 为准，UI 一致性验收以 `frontend/docs/ui-consistency-checklist.md` 为准，异步动效以 `frontend/docs/async-state-transitions.md` 为准。

## Dependency Check

引入任何新库、crate、前端包、SDK 或构建依赖前：

- 从官方包注册表或上游发布源核对当前稳定版本。
- 确认现有依赖或标准工具链不能合理解决问题。
- 明确依赖属于哪个组件，不能为了局部功能把平台依赖引入 shared 或 core。
- 同步更新锁文件、组件文档和代码地图中受影响的依赖方向。
- 不添加未经核验的版本，也不因为示例代码方便而固定到过时版本。

## Implementation Discipline

- 配置、URL、绑定地址、启动和关闭行为变更前，分别读取配置模型、运行网络和平台生命周期文档。
- shared 代码留在 shared crate，HTTP 与业务能力留在 core，平台生命周期留在对应 shell，前端渲染与资源留在 frontend 或平台打包层。
- 模块保持单一职责；存在清晰边界时，不把无关行为堆入同一文件、函数、模块或 crate。
- 删除或简化行为时，同时删除失效的函数、包装器、参数、配置键、测试和文档。
- 除非用户明确要求兼容，不保留没有业务意义的兼容层。
- 修改数据库 schema 或迁移策略前，必须确认是否允许破坏性数据库变更；当前任务已经明确允许时无需重复询问。
- 发现文档、代码地图或注释与当前实现不一致时，在同一改动中修正，不能继续依赖过时描述实施。

## Code Map Check

- 大范围或跨模块实现前，读取 `docs/code-map.md` 总索引和相关结构子地图。
- 新增或移动模块、crate、页面或共享组件，修改公共 API，或改变模块职责时，在同一改动中更新对应子地图。
- 只有新增顶层组件、增加/删除地图文件或改变顶层依赖方向时，才更新 `docs/code-map.md` 根索引。
- 代码地图缺失或明显过时时，先修正地图再继续依赖它实施。
- 根索引和所有子地图统一使用中文，描述当前所有权和依赖，不记录即将废弃的临时结构。

## Comment Check

When adding or modifying code, confirm:

- code comments are written in Chinese
- new or changed non-obvious behavior has a succinct Chinese comment
- existing comments that describe changed behavior are updated
- stale comments are removed or corrected
- comments explain intent, constraints, or ownership instead of restating syntax

If code has no useful surrounding comment and the change affects API behavior, networking, lifecycle, config, persistence, FFI, or platform boundaries, add one.

完整注释要求：

- 每个新源码文件或模块以简短中文文件/模块注释开头，说明它拥有的职责、所属层，以及重要情况下明确不拥有的边界。
- 公共 API 类型、跨模块 struct/enum、数据库 entity、DTO/config struct、repository input struct 和 error enum 使用中文文档注释。
- 数据库 entity、DTO/config struct、repository input struct 和 error enum 的每个字段或枚举项都要说明含义；私有且纯机械字段可以省略。
- 跨所有权边界，或涉及持久化、事务、网络、配置解析、迁移、启动/关闭和安全敏感行为的函数，增加中文文档注释或邻近意图注释，说明使用时机、副作用和重要失败行为。
- 非显然参数、配置键、数据库列、运行模式、路径规则、绑定地址规则和安全敏感值，应在字段、枚举项、函数或邻近位置解释含义。
- 修改注释不足的区域时，补足周围职责和约束，使读者无需从所有调用点反推本地边界。
- 删除只复述语法的注释，不为局部变量和显然分支增加逐行说明。
- 技术名称如 Axum、SQLx、SeaORM、JWT、PRAGMA 以及路径/API 标识可以保留英文，其余解释性文字使用中文。

完成代码改动前，必须在变更源码中搜索 `//`、`///`、`//!` 和块注释标记，核对语言、职责说明和过时内容。

## Verification Matrix

When the relevant code exists, verify:

- local self access through `http://127.0.0.1:<port>`
- LAN access from another device
- remote access through `remote_base_url`
- port conflict behavior
- headless or platform startup behavior
- graceful shutdown behavior
- service status reporting
- frontend artifacts stay out of the Axum crate

## Cargo Verification Scope

Default to the narrowest Cargo command that covers the touched code path.
Small server-shell edits should usually start with:

```text
cargo +stable check -p winestock-server
```

Small shared-library or core-library edits should target the owning package first:

```text
cargo +stable check -p winestock-shared
cargo +stable check -p winestock-core
```

Do not run broad workspace checks as the default AI verification step.
`cargo +stable check --workspace --all-targets` checks every workspace crate, every target, and dev/test dependency paths, so it can invalidate or populate a much larger Cargo fingerprint set than a targeted check.
Use it only when the change is cross-crate, touches public API or dependency/features, affects test-only code, prepares a release/readiness pass, or when the user explicitly asks for full workspace validation.

Formatting checks are separate from compile checks:

```text
cargo +stable fmt --all -- --check
```

Full Rust verification, when justified:

```text
cargo +stable check --workspace --all-targets
cargo +stable test --workspace
cargo +stable build -p winestock-server
```

## Frontend Verification Scope

前端类型与生产构建使用：

```text
cd frontend
pnpm build
```

页面或共享组件改动不能只以构建通过作为完成标准。按 `frontend/docs/ui-consistency-checklist.md` 检查相关业务状态，并至少覆盖：

- 桌面 `1440 × 900`。
- 接近 `768px` 断点的窄桌面或平板视口。
- 移动 `390 × 844`。
- 实际 `getBoundingClientRect()`、计算样式和横向溢出。
- 打开/关闭、空状态、加载、错误、未保存确认等本次受影响的交互状态。
- 浏览器控制台中的新增 error、warning 和 issue。

只改文档且没有影响生成物或源码时，不需要为了形式重复运行前端构建或 Cargo 检查；仍需执行链接、差异和格式审查。

## API 契约核对顺序

判断后端是否存在某个接口、查询参数、请求字段或响应字段时，按以下顺序核对：

1. 优先读取当前运行服务输出的 `/api-docs/openapi.json`；它是当前公开 HTTP 契约的直接证据。
2. 再核对 `core/docs/business-api.md`、`core/docs/business-api/` 和对应领域 API 文档，理解业务语义与权限要求。
3. 只有需要追踪实现、排查契约与行为不一致或修改后端时，才继续查看 controller、service 和 repository 源码。

不要因为前端未调用、页面未展示、手写 TypeScript DTO 缺少字段或源码搜索没有命中预期名称，就判断 API 不存在。删除前端 API 能力或交互入口前，必须先核对 OpenAPI 契约。

仓库不提交静态 `openapi.json`；OpenAPI JSON 由运行中的 core/Axum 服务动态生成。服务已经运行时直接读取当前服务地址下的 `/api-docs/openapi.json`，不需要重新启动服务。

For local API documentation smoke testing, run:

```text
cargo +stable run -p winestock-server
```

The server shell creates `data/config.json` next to the executable with default values if it does not exist.
Debug builds print and expose both `/api-docs/openapi.json` and `/swagger-ui`; Release builds expose only the OpenAPI JSON endpoint.

## Stop Conditions

Stop and ask for direction if:

- a change would make Axum own platform UI assets
- a change requires choosing a frontend framework
- a change requires choosing a persistence engine
- demo structure conflicts with the target architecture
- platform lifecycle policy is ambiguous and affects user-visible behavior
