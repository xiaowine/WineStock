# `core\src` Axum 结构重整实施方案

## 文档状态

本文是 `core\src` 目录结构重整的实施方案文档，用于记录目标边界、分阶段落地顺序和验证要求。

除非被正式规范文档引用，本文不作为日常 agent 的强制约束。

## 背景

当前 `core` 已经不只是“先把 auth 跑起来”的阶段了。  
后续还会持续增加新的 API，例如：

- 搜索数据
- 列表查询
- 详情查看
- 创建/修改业务数据
- 文件或附件相关接口
- 用户、角色、权限管理接口

如果继续只围绕当前 auth 链路做横向拆分，目录会越来越整齐，但**新增 API 时的落点仍然不稳定**。  
这会带来两个长期问题：

1. 新接口不知道应该先按“HTTP 形态”拆，还是按“业务领域”拆。
2. 不同领域的 `search`、`list`、`detail`、`create`、`update` 会不断重复相同的目录结构，最后变成“技术分层清楚，但业务边界还是混在一起”。

## 为什么要推翻上一版拆法

上一版方向的核心问题不是“拆得不够细”，而是**切分维度不对**。

上一版主要是按这些维度在拆：

- `app/`：路由与 OpenAPI 外壳
- `auth/api/handlers`：HTTP handler
- `auth/application`：用例
- `auth/rbac`：权限逻辑

这对于当前 auth 模块本身是有帮助的，但它没有真正解决“以后加别的 API 怎么扩”的问题，因为：

| 问题 | 说明 |
| --- | --- |
| `app/` 语义偏弱 | 它更像 HTTP 外壳，但名字容易让人误解成业务层 |
| state 仍偏 auth | 现在最自然的状态对象仍然是 `AuthRuntime`，这对未来 `stock/search`、`storage`、`user-admin` 都不合适 |
| 切分仍偏“技术横向” | `handlers`、`application` 这些目录描述的是技术层，不是业务领域 |
| 新 API 的归属仍不够自然 | 以后新增“库存搜索”和“库存详情”时，仍然需要先想“放在哪个横向技术层”，而不是直接落到库存领域 |

因此，本方案的核心调整不是“继续细拆 auth”，而是改成：

> **全局只保留 HTTP 外壳和基础设施；业务按领域做纵向切片。**

## 重构目标

本次结构重整目标是：

1. 保持 `core` 仍然是**单个共享 Axum 服务 crate**，不提前拆更多 crate。
2. 保持现有公共 API 和 HTTP 行为稳定：
   - `build_router()`
   - `build_router_with_local_service()`
   - `bootstrap_from_config()`
   - `bind_server()`
   - 当前 auth API
   - OpenAPI JSON 与 Swagger UI 路径
3. 让未来新增 API 时，能直接按**业务领域**落位，而不是继续堆在全局 `handlers` 或顶层模块中。
4. 引入统一的 `CoreState`，避免后续非 auth API 也被迫依赖 `AuthRuntime` 作为根状态。
5. 让全局 HTTP 装配和各领域 API 组织方式清晰分离。

## 非目标

本方案**不**包含以下事项：

- 不拆分 `core` 为多个新 crate。
- 不把服务逻辑搬进 `shared`。
- 不调整 `server`、`shared`、未来 desktop/Android shell 的边界。
- 不修改运行模式、绑定地址规则、配置键或平台职责。
- 不重做 JWT / refresh token / RBAC 业务规则。
- 不为了“先规划未来”而一次性创建很多空目录、空模块和空 trait。

## 结构原则

### 1. 顶层只放全局模块

`core/src/` 顶层只放：

- HTTP 外壳
- 启动和运行时状态
- 持久化基础设施
- 领域模块入口

顶层不应该继续出现：

- `search.rs`
- `detail.rs`
- `handlers/`
- `routes/`

这种按动作或技术层命名、但不表达业务边界的全局模块。

### 2. 业务按领域切，不按 HTTP 动作切

未来新增 API 时：

- “库存搜索”属于 `stock`
- “库存详情”属于 `stock`
- “用户管理”属于 `identity`
- “文件上传/查询”属于 `storage`

**`search` 不是顶层模块名，只能是某个领域里的一个 query 用例。**

### 3. 全局 HTTP 只负责总装配

全局 HTTP 层只负责：

- OpenAPI / Swagger
- 全局 middleware
- merge 各领域 router

它不负责实现具体业务。

### 4. 每个领域内部再做局部分层

领域内部允许有：

- `api`
- `application`
- `domain`

但这些目录只在**领域内部**存在，不在全局横向铺开成一个巨大的技术树。

### 5. 先做一个正确的样板领域，再复制模式

当前已经存在的样板领域其实就是 auth/RBAC/用户注册这一组能力。  
这组能力后续应被收敛成一个更明确的 `identity` 领域，然后未来 `stock`、`storage` 再按同样模式扩展。

## 推荐目标结构

正式推荐结构如下：

```text
core/src/
  lib.rs
  state.rs
  http/
    mod.rs
    router.rs
    docs.rs
  identity/
    mod.rs
    api/
      mod.rs
      router.rs
      handlers/
        login.rs
        logout.rs
        me.rs
        refresh.rs
        register.rs
    application/
      commands/
        login.rs
        logout.rs
        refresh.rs
        register.rs
      queries/
        current_user.rs
      support.rs
    auth/
      runtime.rs
      security.rs
      bootstrap.rs
    rbac/
      bootstrap.rs
      policy.rs
  persistence/
    mod.rs
    connection.rs
    migration/
    entity/
      identity/
    repository/
      identity/
  bootstrap.rs
  server.rs
  tests/
    support.rs
    identity_login.rs
    identity_register.rs
    identity_refresh.rs
    identity_authorization.rs
    bootstrap.rs
    persistence_connection.rs
    persistence_repository.rs
    server.rs
```

未来当库存 API 开始落地时，再新增：

```text
core/src/stock/
  mod.rs
  api/
    mod.rs
    router.rs
    handlers/
      search.rs
      list.rs
      detail.rs
      create.rs
      update.rs
  application/
    queries/
      search.rs
      list.rs
      detail.rs
    commands/
      create.rs
      update.rs
    support.rs
  domain/
    model.rs
    filter.rs
    sort.rs
```

如果未来有文件管理能力，再新增：

```text
core/src/storage/
  ...
```

## 目录含义

### `lib.rs`

- 只保留模块声明、公共 re-export 和稳定入口函数。
- 不直接承担大段 Router 组装、业务路由拼接和 OpenAPI 细节。

### `state.rs`

- 定义全局 `CoreState`。
- 这是后续所有领域共享的 Axum state 根对象。
- 不再让 `AuthRuntime` 直接扮演整个服务的总状态。

### `http/`

- 这是**全局 HTTP 外壳层**，不是业务模块。
- 负责：
  - OpenAPI / Swagger
  - merge 各领域路由
  - 全局 middleware（如果未来真的需要）

推荐不要继续使用 `app/` 这个名字，因为它太容易被理解成业务层。

### `identity/`

- 这是当前 auth/RBAC/注册/当前用户能力的正式业务领域。
- 它不是“纯认证技术模块”，而是“身份与权限”领域。

#### `identity/api/`

- 放这个领域自己的路由与 handler。
- `router.rs` 只负责本领域路由注册和本领域权限中间件挂载。
- 不负责整个服务的总路由。

#### `identity/application/`

- 放领域用例。
- `commands/` 负责会修改状态的行为：
  - `register`
  - `login`
  - `refresh`
  - `logout`
- `queries/` 负责读取型行为：
  - `current_user`

#### `identity/auth/`

- 放 JWT runtime、安全工具和鉴权 bootstrap。
- 它是 identity 领域内部的认证子能力，不再单独作为整个服务的顶层主题。

#### `identity/rbac/`

- 放内置 RBAC bootstrap 和策略约束。
- 这是 identity 领域内部的一部分，不再独立悬挂在 crate 顶层。

### `persistence/`

- 仍然是全局基础设施层。
- 但未来 entity 和 repository 应逐步按领域归组，例如：

```text
persistence/entity/identity/
persistence/entity/stock/
persistence/repository/identity/
persistence/repository/stock/
```

这样领域增长后，持久化层也不会继续横向膨胀。

## 关键架构决策

### 决策 1：把 `auth` 视为 `identity` 领域，而不是全局主题

当前已有能力不只是 token 技术本身，还包括：

- 注册
- 当前用户
- 角色/权限
- 首个管理员初始化
- 条件鉴权

这已经是一个完整业务领域，不再适合继续叫作一个偏技术语义的 `auth` 顶层模块。

### 决策 2：引入 `CoreState`

当前如果把整个服务 state 建在 `AuthRuntime` 之上，后续增加 `stock`、`storage`、`reporting` 时会很别扭。

推荐后续把全局 state 收敛成：

```text
CoreState
  - database / storage runtime
  - identity runtime
  - 未来共享服务状态
```

设计要求：

- `CoreState` 是全局根 state。
- `IdentityRuntime` 只是 `CoreState` 的一部分。
- 各领域 handler 从 `CoreState` 中取自己需要的数据，而不是把某个领域 runtime 伪装成整个系统 runtime。

### 决策 3：全局只 merge 领域 router

未来 `http/router.rs` 的职责应该趋近于：

```text
build_router()
  -> merge docs
  -> merge(identity::api::router(...))
  -> merge(stock::api::router(...))
  -> merge(storage::api::router(...))
```

也就是说：

- 全局 router 不再直接知道每个 endpoint 的业务细节。
- 每个领域自己维护自己的 endpoint 集合。

### 决策 4：不要先造空领域

虽然目标结构里出现了 `stock/`、`storage/`，但它们**只在真正有第一个 API 落地时再创建**。

不要为了“看起来规划完整”而提前生成大量空目录和空模块。

当前应该先把：

- `http`
- `state`
- `identity`

这一套模式建立正确。

## 新增 API 时的落位规则

以后新增 API 时，按下面规则处理。

### 示例 1：新增“库存搜索 API”

例如新增：

```text
GET /api/stock/search
```

推荐落位：

```text
stock/api/handlers/search.rs
stock/api/router.rs
stock/application/queries/search.rs
stock/domain/filter.rs
persistence/repository/stock/...
```

不应该落位到：

- `core/src/search.rs`
- `core/src/api/search.rs`
- `core/src/handlers/search.rs`

### 示例 2：新增“库存详情 API”

例如新增：

```text
GET /api/stock/:id
```

推荐落位：

```text
stock/api/handlers/detail.rs
stock/application/queries/detail.rs
stock/domain/model.rs
```

### 示例 3：新增“库存修改 API”

例如新增：

```text
PATCH /api/stock/:id
```

推荐落位：

```text
stock/api/handlers/update.rs
stock/application/commands/update.rs
stock/domain/model.rs
persistence/repository/stock/...
```

## 分阶段实施计划

建议按下面顺序推进，每一阶段都能独立提交并验证。

### 阶段 0：回退错误方向，恢复当前稳定基线

#### 目标

先回退上一版“按横向技术层拆 auth”的结构调整，回到当前稳定代码基线。

#### 原因

如果不先回退，后续实施新的领域切片方案时，会把两条不同方向的结构变更叠在一起，评审和回归都会变得非常困难。

#### 任务

1. 回退 `app/` 相关变更。
2. 回退 `auth/api/handlers`、`auth/application` 这类上一版横向分层目录。
3. 恢复当前代码结构。
4. 仅保留新的实施方案文档，不保留错误方向的结构性代码改动。

#### 完成标准

- 代码回到本轮结构尝试前的状态。
- 文档转向新的领域切片方案。

### 阶段 1：先引入 `http/` 与 `CoreState`，不急着改业务目录

#### 目标

先把“全局外壳”和“全局状态根对象”立起来，但暂时不做 `auth -> identity` 重命名。

#### 任务

1. 新建 `core/src/state.rs`，定义 `CoreState`。
2. 把当前路由组装和 OpenAPI 从 `lib.rs` 移到 `core/src/http/`。
3. `lib.rs` 只保留：
   - 模块声明
   - re-export
   - `build_router*`
4. 让 `build_router_with_local_service()` 最终面向 `CoreState` 组装 Router。

#### 关键要求

- 这一阶段只建立全局骨架。
- 不在这一阶段顺手重写 auth 业务逻辑。

#### 完成标准

- `lib.rs` 变薄。
- `http/` 成为唯一的全局 HTTP 外壳。
- `AuthRuntime` 不再是默认的全局 state 语义中心。

### 阶段 2：把当前 auth 正式收敛为 `identity` 领域

#### 目标

把当前 auth/RBAC/注册/当前用户这一组能力重命名并归位到 `identity/`。

#### 任务

1. 设计 `identity/` 模块边界。
2. 把当前：
   - 登录
   - 注册
   - refresh
   - logout
   - 当前用户
   - RBAC bootstrap
   - JWT runtime
   - security helper
   收拢到 `identity/`。
3. `identity/api/router.rs` 负责 identity 领域自己的路由。
4. `http/router.rs` 只 merge `identity::api::router(...)`。

#### 命名建议

- 当前的 `auth/runtime.rs` -> `identity/auth/runtime.rs`
- 当前的 `rbac.rs` -> `identity/rbac/bootstrap.rs`

#### 完成标准

- `identity` 成为第一个正式领域样板。
- 顶层不再把 auth 当成整个系统的唯一中心主题。

### 阶段 3：把 identity 领域内部再做局部分层

#### 目标

在 `identity` 领域内部做“API / 用例 / 认证子能力 / RBAC 子能力”拆分，但只限于这个领域内部。

#### 任务

1. 建立 `identity/api/`：
   - `router.rs`
   - `handlers/`
2. 建立 `identity/application/`：
   - `commands/`
   - `queries/`
   - `support.rs`
3. 建立 `identity/auth/`：
   - `runtime.rs`
   - `security.rs`
   - `bootstrap.rs`
4. 建立 `identity/rbac/`：
   - `bootstrap.rs`
   - 如有必要再补 `policy.rs`

#### 关键要求

- 这是**领域内部分层**，不是把整个 `core` 再次横向技术分层。
- 不要在 `core/src/` 顶层重新出现全局 `handlers`、`commands`、`queries` 目录。

#### 完成标准

- 登录/注册/刷新/登出/当前用户不再集中在单文件中。
- identity 领域内部职责清楚，但全局目录仍然按领域组织。

### 阶段 4：按领域重整持久化层

#### 目标

让 `persistence/` 也逐步跟随领域结构演进，避免以后所有 entity/repository 再次堆在一起。

#### 任务

1. 保持 `connection.rs`、`migration/` 作为全局基础设施。
2. 将 entity/repository 逐步按领域归组：
   - `entity/identity/`
   - `repository/identity/`
3. 后续 stock 落地时继续按同样方式扩展：
   - `entity/stock/`
   - `repository/stock/`

#### 注意事项

- 不要求第一步就重写所有 SQL。
- 允许复杂查询继续保留显式 SQL，只要边界属于对应领域的 repository。

#### 完成标准

- 持久化层不会在未来因业务扩展再次横向失控。

### 阶段 5：建立“下一个领域”的接入样板

#### 目标

当第一个非 identity 的业务 API 开始实现时，用它来验证整个结构是否真的能扩展。

最推荐的下一个领域就是：

- `stock`

因为你已经明确提到了未来会有：

- 搜索数据
- 查看数据
- 列表/详情类接口

#### 任务

1. 当第一个库存 API 开始实现时，再创建 `stock/`。
2. 首批最有代表性的样板接口建议是：
   - `GET /api/stock/search`
   - `GET /api/stock/:id`
3. 在 `stock` 中先落完整链路：
   - `api/handlers`
   - `application/queries`
   - `domain/filter` / `domain/model`
   - `repository/stock`

#### 完成标准

- `identity` 之外的第二个领域能自然接入。
- 不需要为新增业务 API 再去改全局目录结构。

### 阶段 6：测试按领域和全局外壳拆分

#### 目标

测试也要跟着新的领域边界走，避免测试仍停留在旧结构。

#### 推荐方向

```text
tests/support.rs
tests/identity_login.rs
tests/identity_register.rs
tests/identity_refresh.rs
tests/identity_authorization.rs
tests/http_openapi.rs
```

以后再加：

```text
tests/stock_search.rs
tests/stock_detail.rs
```

#### 关键要求

- 测试命名直接体现领域和行为。
- 不保留一个持续膨胀的“大一统 auth 测试文件”。

### 阶段 7：更新文档和代码地图

#### 目标

在结构稳定后，把文档同步到位，避免“代码已经按领域切片，文档还停留在 auth 横向拆分”。

#### 任务

1. 更新 `docs/code-map.md`
2. 更新 `core/docs/rbac-permission-model.md`
3. 如有需要，补充：
   - identity 领域说明
   - stock API 结构说明
4. 审核新增/修改源文件中的中文注释

## 建议 PR 切片

不要把所有阶段塞进一个 PR。推荐拆成：

1. **PR 1：回退错误方向 + 更新方案文档**
2. **PR 2：引入 `http/` 和 `CoreState`**
3. **PR 3：`auth` 收敛为 `identity` 领域**
4. **PR 4：identity 内部分层**
5. **PR 5：持久化层按 identity 归组**
6. **PR 6：第一个 stock API 样板**
7. **PR 7：测试与文档收尾**

这样每个 PR 都有单一主题，评审也更容易判断：

- 是“全局骨架变化”
- 还是“领域归位”
- 还是“新增一个业务领域样板”

## 验证策略

### 每阶段最小验证

- `cargo check --workspace --all-targets`
- `cargo test --workspace`

### 关键阶段额外验证

在阶段 1、阶段 2、阶段 3、阶段 5 完成后，额外确认：

1. `POST /api/auth/register`
2. `POST /api/auth/login`
3. `POST /api/auth/refresh`
4. `POST /api/auth/logout`
5. `GET /api/auth/me`
6. `/api-docs/openapi.json`
7. `/swagger-ui`

当 `stock` 首批 API 落地后，再新增验证：

8. `GET /api/stock/search`
10. `GET /api/stock/:id`

### 必须保留的行为断言

- `client-only` 不打开本地数据库。
- `self-hosted` / `server-mode` 会初始化本地服务依赖。
- 首个用户自动成为 `admin`。
- 后续注册受 `user.register` 权限约束。
- 当前权限判断以数据库最新状态为准。
- refresh token 轮换和旧 token 复用检测不回退。
- 端口冲突仍返回明确错误。

## 风险与规避

### 风险 1：过早建立过多空领域

规避方式：

- 只先做 `identity`。
- `stock`、`storage` 在第一个真实 API 落地时再创建。

### 风险 2：只改名字，不改根状态模型

如果只把 `auth` 改名成 `identity`，但仍让 `AuthRuntime` 充当全局 state，本质问题没有解决。

规避方式：

- 必须引入 `CoreState`。
- 必须让 `http/router.rs` 面向 `CoreState` merge 各领域路由。

### 风险 3：继续把“技术层目录”铺成全局结构

如果出现：

- `core/src/handlers`
- `core/src/queries`
- `core/src/commands`

说明结构又回到了横向技术分层，而不是领域切片。

规避方式：

- 这些目录只能存在于某个领域内部。

### 风险 4：为了未来 API 过度抽象

规避方式：

- 不先建抽象 trait 工厂。
- 不先建空的通用 service 框架。
- 先用 `identity` 验证模式，再用 `stock` 复制模式。

## 完成定义

当以下条件同时满足时，可认为本轮结构整治完成：

1. `lib.rs` 不再承载主要 router 细节。
2. `http/` 成为唯一的全局 HTTP 外壳。
3. 存在统一的 `CoreState`。
4. 当前 auth/RBAC/注册/当前用户能力已正式收敛为 `identity` 领域。
5. 全局目录按领域组织，而不是按 `handlers` / `queries` / `search` 这类横向技术主题组织。
6. 新增第二个领域（优先 `stock`）时，不需要重新调整顶层结构。
7. `docs/code-map.md` 已同步更新。
8. 所有既有 API 行为、配置语义和网络边界保持不变。

## 实施顺序建议

如果只打算做一轮真正适合后续扩展的整治，优先顺序建议固定为：

1. **先回退错误方向**
2. **先立全局骨架：`http` + `CoreState`**
3. **再把当前 auth 收敛成 `identity` 领域**
4. **再在 identity 内部做局部分层**
5. **再让持久化层跟随领域归组**
6. **最后用 `stock` 作为第二个领域样板验证结构**

一句话总结：

> **后续会不断增加 API 时，`core` 的正确方向不是继续按 handler 或 app 细拆，而是“全局 HTTP 外壳 + 统一 CoreState + 按领域纵向切片”。**
