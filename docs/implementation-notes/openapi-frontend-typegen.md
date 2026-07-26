# 前端 API 类型从 OpenAPI 生成方案

本方案解决前端手写 TypeScript DTO 与 core OpenAPI 契约缺乏编译期同步信号的问题。
它是跨组件方案：core 增加开发期契约导出通道，frontend 引入类型生成与桥接层；不改变任何运行时 HTTP 行为。

## 背景与问题

- core 通过 utoipa 在 Debug 构建动态生成 `/api-docs/openapi.json`（`core/src/http/docs.rs`），Release 不暴露；仓库不提交静态 `openapi.json`。
- `frontend/src/api/*.ts` 约二十个模块的请求/响应 DTO 全部手写，仅 `items.ts` 就有三百余行接口声明。
- 契约同步目前只靠 `docs/agent-checklist.md` 的“API 契约核对顺序”人工纪律；后端字段增删改在前端没有任何编译期信号，属于结构性漂移风险。

## 目标与非目标

目标：

- 后端 HTTP 契约变化时，前端通过重新生成类型获得 `vue-tsc` 编译期报错。
- 前端构建（含 Android Gradle 调用的 `build:android`）不依赖 Rust 工具链。
- 保持现有 `frontend/src/api/*.ts` 模块结构、导出名称和请求函数不变，页面代码零改动。

非目标：

- 不引入生成的请求客户端（如 openapi-fetch）；请求函数仍由 `api/*.ts` 手写并复用 `apiClient`。
- 不改变“仓库不提交 `openapi.json`”的策略；OpenAPI JSON 仍是不入库的中间产物。
- 不覆盖前端本地专有类型（上传进度、草稿模型等非 HTTP 契约类型仍手写）。

## 总体设计

```text
core (Debug)                         frontend
ApiDoc::openapi()                    openapi-typescript CLI
  │ core/examples/dump_openapi.rs      │ pnpm gen:api-types
  ▼                                    ▼
target/openapi/openapi.json  ──────► src/api/generated/schema.d.ts（入库）
     （不入库的中间产物）               │ type 别名桥接
                                       ▼
                              src/api/*.ts 现有模块（导出名不变）
```

### 1. core：开发期契约导出通道

- `core/src/http/docs.rs` 增加 Debug-only 公开函数（示意）：

  ```rust
  /// 返回当前 OpenAPI 文档 JSON，仅供开发期类型生成工具使用。
  #[cfg(debug_assertions)]
  pub fn openapi_document_json() -> String
  ```

  经 `core/src/http/mod.rs` 和 `core/src/lib.rs` 以 `#[cfg(debug_assertions)]` 重新导出。
- 新增 `core/examples/dump_openapi.rs`：接收一个输出路径参数（默认 `target/openapi/openapi.json`），把文档 JSON 写入该文件。examples 默认走 dev profile，天然满足 `debug_assertions` 门控，不影响 Release 制品。
- 备选方案“从运行中的 Debug 服务抓取”被否决：需要手动起服务并协调端口，且结果依赖运行环境；example 导出是离线、确定性的。

### 2. frontend：类型生成与入库

- 新增 devDependency `openapi-typescript`（实施时以 npm 最新稳定版为准，当前为 7.13.0；需要 Node ≥ 20，与现有工具链兼容）。
- `frontend/package.json` 新增脚本：

  ```jsonc
  {
    // 仅执行代码生成：读取工作区 target 下的中间 JSON
    "gen:api-types": "openapi-typescript ../target/openapi/openapi.json -o src/api/generated/schema.d.ts",
    // 完整链路：先让 core 导出契约，再生成类型
    "gen:api": "cargo run -p winestock-core --example dump_openapi && pnpm gen:api-types"
  }
  ```

- 生成产物 `frontend/src/api/generated/schema.d.ts` **提交入库**：
  - 前端 Web/Android 构建因此不需要 Rust 工具链，只有后端契约变化时才需要重新生成。
  - 生成是确定性的，diff 直接呈现契约变化，便于评审。
  - 该目录只允许生成器写入，不接受手工编辑；目录内放置 README 或文件头注释声明这一约束。

### 3. 桥接层：类型别名替换手写接口

现有模块逐个把手写接口体替换为对生成类型的别名，导出名与模块归属不变（示意）：

```ts
// frontend/src/api/items.ts
import type { components } from "./generated/schema";

export type ItemEditorResponse = components["schemas"]["ItemEditorResponse"];
export type ItemMutationResponse = components["schemas"]["ItemMutationResponse"];
```

- 请求函数、查询参数拼装和 `apiClient` 调用保持手写；`paths` 类型留作后续可选增强，不在本方案范围。
- utoipa 泛型 schema（如 `PaginatedResponse<UserAdminResponse>`）在 OpenAPI 中有展开后的组件名，桥接别名负责映射回现有导出名（如 `api/pagination.ts` 的泛型分页接口可保留手写泛型壳，成员类型指向生成类型）。
- 桥接过程中发现“手写比 OpenAPI 更窄”（例如手写字面量联合而 OpenAPI 是普通 string）时，一律在 Rust 侧补齐 schema 标注（枚举派生、`#[schema(...)]`），不得在前端保留手工收窄——这类差异正是本方案要消除的漂移。
- 确属前端本地的类型（文件预检、草稿、上传进度）保持手写，不迁移。

### 4. 漂移防线与流程规则

- `docs/agent-checklist.md`：
  - “API 契约核对顺序”增加一条：`frontend/src/api/generated/schema.d.ts` 是契约的派生证据，判断接口存在性仍以 OpenAPI/业务文档为准。
  - 新增规则：修改 core HTTP 契约（路径、DTO、查询参数、错误响应）的改动，必须在同一改动中运行 `pnpm gen:api` 并提交生成产物；验证方式为重新生成后 `git diff --exit-code frontend/src/api/generated/`。
  - “仓库不提交静态 `openapi.json`”条目补充说明：该策略不变，生成的 `.d.ts` 是允许并要求入库的派生产物。
- 代码地图同步：`docs/code-map/core.md`（examples 导出工具）、`docs/code-map/frontend.md`（generated 目录与生成脚本）、`docs/code-map/workspace.md`（验证入口）。

## 实施阶段

每个阶段可独立落地并验证：

1. **阶段 0：基础设施。** core 导出函数 + `dump_openapi` example + frontend 依赖与脚本 + generated 目录；此时生成产物入库但尚无模块引用。
2. **阶段 1：试点模块。** 迁移 `api/items.ts`（最大、字段最活跃）；核对生成类型与手写类型 diff，把发现的 schema 精度问题回补到 Rust 侧；`pnpm build` 通过。
3. **阶段 2：全量桥接。** 按模块逐个迁移其余 `api/*.ts`；每个模块迁移后跑 `pnpm build`，出现的类型错误即真实契约差异，逐一归因（前端错 → 改前端；schema 粗 → 改 Rust 标注）。
4. **阶段 3：流程固化。** 更新 agent-checklist、AGENTS.md 相关表述与三份代码地图；补充本文件的验收记录。

## 风险与边界

- **生成器输出格式随版本变化**：锁定 devDependency 精确版本于 lockfile；升级生成器时单独提交，diff 只含格式性变化。
- **OpenAPI 组件命名冲突或改名**：utoipa 组件名来自 Rust 类型名，改名会连带生成类型名变化；桥接别名层正是为吸收这类变化而存在。
- **`ApiErrorResponse`/错误契约**：`api/errors.ts` 的错误解析逻辑保持手写，仅其结构类型指向生成类型。
- **Windows 环境**：脚本使用相对路径与 pnpm/cargo 原生命令，无 shell 特定语法，Windows/CI 通用。

## 验收标准

- `cargo run -p winestock-core --example dump_openapi` 离线产出合法 OpenAPI 3.1 JSON；`cargo +stable check -p winestock-core` 通过且 Release 构建不包含新增导出。
- `pnpm gen:api` 全链路可重复执行且输出确定；`pnpm build`、`pnpm build:android` 在无 Rust 工具链的前提下成功。
- 抽样验证：在 core 侧临时增删一个响应字段，重新生成后前端 `vue-tsc` 能对使用点报错。
- 迁移完成的模块中不存在与生成 schema 重复的手写接口体；前端页面代码无改动。
