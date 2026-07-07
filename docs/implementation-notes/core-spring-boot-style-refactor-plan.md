# `core\src` Spring Boot 风格结构重整实施方案

## 文档状态

本文原本是 `core\src` 下一轮结构重整的实施方案文档，用于记录目标边界、阶段顺序和验证要求。

截至当前代码状态，本方案已完成落地：`core\src` 顶层已收敛为 `http / security / auth / users / rbac / persistence`，并保留既有 HTTP 路径与启动入口不变。

本文保留为本轮重整的历史实施记录；当前正式源码布局与职责边界以 `docs/code-map.md` 为准。

除非被正式规范文档引用，本文不作为日常 agent 的强制约束；但如果后续任务明确要求“按实施方案执行”，则应以本文为准。

## 背景

当前 `core` 已经完成了第一轮方向校正：

- 顶层已经有 `http/`
- 已经引入统一的 `CoreState`
- 当前 auth/RBAC/注册/当前用户能力已先收拢到 `identity/`

这一步解决了“全局到处散着 handler 和路由”的问题，但新的结构仍然存在一个可读性问题：

> **顶层方向已经对了，但领域内部分层对当前项目体量来说还是偏重。**

现在的主要不顺点不是功能错误，而是“读起来不够直白”：

| 现状 | 问题 |
| --- | --- |
| `identity` 作为总包 | 语义偏抽象，既像业务领域，又混入前置安全能力 |
| `api / application / auth / rbac` 并存 | 一次引入了多套切分维度，阅读成本高 |
| `commands / queries / handlers` | 对当前规模偏重，不够接近“一个业务一个服务”的直觉 |
| 安全前置逻辑仍在 `identity` 内部 | 容易让“认证/授权横切层”和“身份业务”继续缠在一起 |

用户已经明确认可下一步的方向：

1. **前置安全应该单独存在**
2. **每个业务模块应该可以独立**
3. **每个业务模块内部优先用 `controller + service` 表达**
4. **认证只负责判断有没有权限，具体操作仍交给业务服务**

因此，下一轮不是再细化 `identity`，而是：

> **保留全局 HTTP 外壳和 `CoreState`，把 `identity` 继续拆解成 `security + auth + users + rbac`，并让业务模块采用更接近 Spring Boot 的 `controller + service` 结构。**

## 为什么参考 Spring Boot 风格

这里说的“参考 Spring Boot”，不是照搬 Java class 形式，而是借用它最有价值的两点：

1. **按业务模块组织，而不是按全局技术层组织**
2. **业务模块内部优先使用直白的 `controller + service + repository` 语义**

Spring Boot 常见的可读性优势是：

- 打开目录就能先看到业务边界
- 每个业务模块内部有稳定的入口文件
- 前置安全、业务编排、数据访问职责清楚

对当前 Rust/Axum 项目来说，更合适的翻译方式是：

- 用 `mod.rs + controller.rs + service.rs` 代替 Java class 组合
- 用 `security/` 代替散落在业务模块中的前置安全逻辑
- 用 `repository` 保持持久化边界
- **不默认保留 `commands / queries / handlers` 这种更重的分层**

## 本轮目标

本轮结构重整的目标是：

1. 保持 `core` 仍然是**单个共享 Axum 服务 crate**
2. 保持已有稳定入口和 HTTP 行为不变：
   - `build_router()`
   - `build_router_with_local_service()`
   - `bootstrap_from_config()`
   - `bind_server()`
   - `/api/health`
   - `/api/auth/register`
   - `/api/auth/login`
   - `/api/auth/refresh`
   - `/api/auth/logout`
   - `/api/auth/me`
   - OpenAPI JSON 与 Swagger UI 路径
3. 保留 `http/` 作为唯一全局 HTTP 外壳
4. 保留 `CoreState` 作为统一服务级共享状态
5. 把当前 `identity` 内的职责进一步拆开：
   - `security/`：认证与授权横切层
   - `auth/`：登录/刷新/登出等会话认证业务
   - `users/`：注册、当前用户、未来用户管理业务
   - `rbac/`：角色权限模型、内置定义与策略常量
6. 让后续新业务模块优先按 `controller + service` 模式接入，而不是继续复制 `api / application / auth / rbac`

## 非目标

本方案**不**包含以下事项：

- 不拆分 `core` 为多个 crate
- 不把服务逻辑搬进 `shared`
- 不改 `server`、`shared`、desktop、Android shell 的边界
- 不修改运行模式、绑定地址规则、配置键或平台职责
- 不重做 JWT、refresh token、RBAC 业务规则本身
- 不修改现有 HTTP 路径，只因为目录名发生变化
- 不把当前项目改成多个网络微服务

## 保留项

以下结构和决定应当保留，不是这轮要推翻的对象：

### 1. 保留 `http/`

`http/` 已经是正确方向：

- 放全局 health endpoint
- 放 OpenAPI / Swagger
- 放总 router 装配
- 不承担具体业务规则

这一层不需要再退回去。

### 2. 保留 `CoreState`

`CoreState` 仍然应该是全局共享状态根对象。

核心原则保持不变：

- **服务级状态属于 `CoreState`**
- **业务模块自己的 runtime 只是 `CoreState` 的一部分**
- **不能再让某个业务模块 runtime 伪装成整个系统 state**

目标形态可以收敛成：

```text
CoreState
  - storage / database
  - security runtime
  - 其他服务级共享状态
```

### 3. 保留“按业务增长”的大方向

当前不再应该回到：

- 全局 `handlers/`
- 全局 `commands/`
- 全局 `queries/`
- 全局 `search.rs`

后续新增业务仍然要按业务模块落位，例如：

- `auth`
- `users`
- `stock`
- `storage`

## 结构原则

### 1. `security` 是横切层，不是业务域

`security/` 只负责：

- bearer token 解析
- 当前用户提取
- JWT 签发/校验支持
- 密码哈希与校验
- refresh token 哈希和安全随机数
- 权限校验 middleware

它**不**负责：

- 登录业务流程
- 注册业务流程
- 当前用户响应组装
- 角色权限管理业务
- 库存等业务操作

一句话：

> **`security` 只决定“能不能进入业务”，不决定“业务具体怎么做”。**

### 2. `auth` 是会话认证业务，不是全局安全层

`auth/` 负责：

- login
- refresh
- logout
- auth bootstrap（数据库托管设置、签名密钥初始化）

它是业务模块，不是全局前置层。

### 3. `users` 是用户业务模块

`users/` 负责：

- register
- me
- 未来的 user manage / user detail / user list

即使某些路径现在仍然是 `/api/auth/register` 或 `/api/auth/me`，实现也可以归到 `users/`。

> **URL 兼容性不需要和模块名完全一致。**

### 4. `rbac` 是授权模型模块

`rbac/` 负责：

- 内置角色/权限定义
- 启动时补齐 RBAC 基础数据
- 稳定权限常量与策略约束

如果未来出现角色/权限管理 API，再决定是否给 `rbac/` 增加 `controller.rs` 和 `service.rs`。

### 5. 每个业务模块先从 `controller.rs + service.rs` 起步

默认形态应尽量简单：

```text
users/
  mod.rs
  controller.rs
  service.rs
  permissions.rs
```

也就是说：

- 初期不默认上 `handlers/`
- 初期不默认上 `commands/queries`
- 初期不默认上很多空目录

只有在某个模块真的变大时，再在**模块内部**继续细分。

### 6. `repository` 只做数据访问

repository 层负责：

- 查询
- 插入/更新/删除
- 事务边界

它不负责：

- token 解析
- HTTP 响应
- 业务权限判断
- handler 级流程编排

## 推荐目标结构

推荐目标结构如下：

```text
core/src/
  lib.rs
  bootstrap.rs
  state.rs
  server.rs

  http/
    mod.rs
    router.rs
    health.rs
    docs.rs

  security/
    mod.rs
    middleware.rs
    current_user.rs
    jwt.rs
    password.rs
    token.rs
    error.rs

  auth/
    mod.rs
    controller.rs
    service.rs
    bootstrap.rs

  users/
    mod.rs
    controller.rs
    service.rs
    permissions.rs

  rbac/
    mod.rs
    bootstrap.rs
    policy.rs

  persistence/
    mod.rs
    connection.rs
    migration/
    entity/
      mod.rs
      auth_setting.rs
      auth_signing_key.rs
      refresh_token.rs
      user.rs
      file_object.rs
    repository/
      mod.rs
      time.rs
      auth_repo.rs
      user_repo.rs
      rbac_repo.rs
      refresh_token_repo.rs
      file_object_repo.rs

  tests/
    support.rs
    http_health.rs
    http_openapi.rs
    security_authorization.rs
    auth_login.rs
    auth_refresh.rs
    auth_logout.rs
    users_register.rs
    users_me.rs
    bootstrap.rs
    persistence_connection.rs
    persistence_repository.rs
    server.rs
```

未来第一个新的正式业务样板建议是：

```text
stock/
  mod.rs
  controller.rs
  service.rs
  permissions.rs
```

## 模块职责说明

### `http/`

全局 HTTP 外壳层，只负责：

- `GET /api/health`
- OpenAPI / Swagger
- merge 各业务 router
- 全局 middleware 装配点

不负责业务实现。

### `security/`

全局认证与授权前置层，只负责：

- token 解析
- 当前用户提取
- JWT 校验与签发支持
- 权限判断

不负责 login/register/logout 业务。

### `auth/`

会话认证业务模块，负责：

- 登录
- refresh
- logout
- auth bootstrap

它可以调用 `security/` 提供的能力，但它本身不是横切层。

### `users/`

用户业务模块，负责：

- register
- me
- 未来用户管理能力

它应当通过 `security` 获取当前用户上下文，但不直接承担 token 解析。

### `rbac/`

授权模型模块，负责：

- 内置角色与权限定义
- 策略常量
- 启动补齐

### `persistence/`

全局基础设施层，负责：

- SQLite 连接
- migration
- entity
- repository

对当前规模，`repository` 推荐直接使用更直白的文件名：

- `auth_repo.rs`
- `user_repo.rs`
- `rbac_repo.rs`
- `refresh_token_repo.rs`

而不是再挂一个 `identity/` 总目录。

## 请求链路

### 受保护业务接口

例如未来的库存接口：

```text
http/router
  -> security/middleware
  -> stock/controller
  -> stock/service
  -> stock_repo
```

### 登录/刷新/登出

```text
http/router
  -> auth/controller
  -> auth/service
  -> security helpers + repository
```

### 注册/当前用户

```text
http/router
  -> 条件安全校验（需要时）
  -> users/controller
  -> users/service
  -> user_repo / rbac_repo / auth_repo
```

## 当前结构到目标结构的映射

| 当前结构 | 目标结构 |
| --- | --- |
| `identity/auth/runtime.rs` | `security/jwt.rs` + `security/current_user.rs` |
| `identity/auth/authorization.rs` | `security/middleware.rs` |
| `identity/auth/security.rs` | `security/password.rs` + `security/token.rs` |
| `identity/auth/bootstrap.rs` | `auth/bootstrap.rs` |
| `identity/api/handlers/login.rs` + `application/commands/login.rs` | `auth/controller.rs` + `auth/service.rs` |
| `identity/api/handlers/refresh.rs` + `application/commands/refresh.rs` | `auth/controller.rs` + `auth/service.rs` |
| `identity/api/handlers/logout.rs` + `application/commands/logout.rs` | `auth/controller.rs` + `auth/service.rs` |
| `identity/api/handlers/register.rs` + `application/commands/register.rs` | `users/controller.rs` + `users/service.rs` |
| `identity/api/handlers/me.rs` + `application/queries/current_user.rs` | `users/controller.rs` + `users/service.rs` |
| `identity/rbac/bootstrap.rs` | `rbac/bootstrap.rs` |
| `identity/rbac/policy.rs` | `rbac/policy.rs` |
| `persistence/repository/identity/auth.rs` | `persistence/repository/auth_repo.rs` |
| `persistence/repository/identity/user.rs` | `persistence/repository/user_repo.rs` |
| `persistence/repository/identity/rbac.rs` | `persistence/repository/rbac_repo.rs` |
| `persistence/repository/identity/refresh_token.rs` | `persistence/repository/refresh_token_repo.rs` |

## 分阶段实施计划

建议按下面顺序推进，每一阶段都可以独立验证和提交。

### 阶段 0：冻结当前基线

#### 目标

先把当前 `http/ + CoreState + identity` 结构视为迁移起点，不在同一个 PR 里同时做新的业务功能和新的结构切分。

#### 完成标准

- 当前稳定行为可用
- 重构只围绕结构，不夹带新业务特性

### 阶段 1：先抽出 `security/`

#### 目标

先把“全局前置安全”从 `identity` 中抽出来。

#### 任务

1. 新建 `core/src/security/`
2. 移动或拆分：
   - JWT 签发/校验
   - `CurrentUser`
   - bearer token 提取
   - 权限 middleware
   - 密码哈希/校验
   - refresh token hash/random helper
3. 让业务模块只依赖 `security` 提供的上下文与校验能力

#### 关键要求

- `security` 只做认证/授权
- `login/register/logout/me` 不要误搬进 `security`

### 阶段 2：拆出 `auth/`

#### 目标

把 login/refresh/logout 和 auth bootstrap 收敛成独立业务模块。

#### 任务

1. 新建 `core/src/auth/`
2. 建立：
   - `controller.rs`
   - `service.rs`
   - `bootstrap.rs`
3. 把当前 login/refresh/logout 逻辑迁移到 `auth/service.rs`
4. 把 `/api/auth/login`、`/api/auth/refresh`、`/api/auth/logout` 的 HTTP 入口迁移到 `auth/controller.rs`

#### 完成标准

- `auth` 只表达会话认证业务
- 不再把登录等业务留在 `security` 或 `identity`

### 阶段 3：拆出 `users/`

#### 目标

把 register/me 和未来用户管理能力收敛成独立业务模块。

#### 任务

1. 新建 `core/src/users/`
2. 建立：
   - `controller.rs`
   - `service.rs`
   - `permissions.rs`
3. 把 register/me 的业务逻辑迁移到 `users/service.rs`
4. 保持 `/api/auth/register` 和 `/api/auth/me` 路径不变

#### 注意事项

- 模块名可以是 `users`，但 HTTP 路径不必同步改名
- `me` 即使是读取逻辑，也直接归在 `users/service.rs`，不强制再拆 `queries`

### 阶段 4：顶层拉平 `rbac/`

#### 目标

把 RBAC 从 `identity` 内部子目录提升成顶层授权模型模块。

#### 任务

1. 新建或迁移到 `core/src/rbac/`
2. 保留：
   - `bootstrap.rs`
   - `policy.rs`
3. 后续若出现角色/权限管理 API，再考虑增加 `controller.rs` / `service.rs`

### 阶段 5：整理持久化命名

#### 目标

让 `persistence/` 的命名也与新结构对齐，避免保留“已经没有 `identity`，仓储目录里还挂着 `identity/`”的中间态。

#### 任务

1. 将 `repository/identity/*.rs` 收敛为更直白的文件名：
   - `auth_repo.rs`
   - `user_repo.rs`
   - `rbac_repo.rs`
   - `refresh_token_repo.rs`
2. 评估 entity 是否也需要同步拉平；如果只是表字段映射，允许保持扁平文件结构

#### 原则

- 数据库表名不变
- migration 不因目录名变化而重写
- 只改代码结构和调用路径

### 阶段 6：测试和文档跟随重命名

#### 目标

让测试与文档命名也反映新的模块结构。

#### 任务

1. 测试文件改为：
   - `security_authorization.rs`
   - `auth_login.rs`
   - `auth_refresh.rs`
   - `auth_logout.rs`
   - `users_register.rs`
   - `users_me.rs`
2. 更新：
   - `docs/code-map.md`
   - `docs/rbac-permission-model.md`
   - 本实施方案文档引用关系

### 阶段 7：用 `stock` 作为第二个正式业务样板

#### 目标

用下一个真实业务域验证新结构是否自然扩展。

#### 任务

1. 当第一个库存 API 开始实现时，再创建 `stock/`
2. 首批最合适的样板接口：
   - `GET /api/stock/search`
   - `GET /api/stock/:id`
3. `stock/` 先采用：
   - `controller.rs`
   - `service.rs`
   - `permissions.rs`

#### 完成标准

- 新业务模块不再需要复制 `identity/api/application/auth/rbac`
- 新业务可直接按 `controller + service` 模式接入

## 推荐 PR 切片

不要把所有阶段塞进一个 PR。推荐拆成：

1. **PR 1：实施方案文档**
2. **PR 2：抽出 `security/`**
3. **PR 3：拆出 `auth/`**
4. **PR 4：拆出 `users/`**
5. **PR 5：顶层拉平 `rbac/` 与 `persistence` 命名**
6. **PR 6：测试和文档收尾**
7. **PR 7：`stock` 业务样板**

## 验证策略

### 每阶段最小验证

- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo build -p winestock-server`

### 关键阶段额外验证

在阶段 1 到阶段 5 完成后，额外确认：

1. `GET /api/health`
2. `POST /api/auth/register`
3. `POST /api/auth/login`
4. `POST /api/auth/refresh`
5. `POST /api/auth/logout`
6. `GET /api/auth/me`
7. `/api-docs/openapi.json`
8. `/swagger-ui`

### 必须保留的行为断言

- `client-only` 不打开本地数据库
- `self-hosted` / `server-mode` 会初始化本地服务依赖
- 首个用户自动成为 `admin`
- 后续注册受 `user.register` 权限约束
- 当前权限判断以数据库最新状态为准
- refresh token 轮换和旧 token 复用检测不回退
- 端口冲突仍返回明确错误

## 风险与规避

### 风险 1：把 `security` 和 `auth` 再次混回去

规避方式：

- `security` 只保留认证/授权前置能力
- login/refresh/logout 统一归 `auth/service.rs`

### 风险 2：为保持目录整齐而改掉 HTTP 路径

规避方式：

- 明确“模块名”和“URL 路径”可以解耦
- 先保留兼容路径，再讨论产品级 API 改版

### 风险 3：过早继续细拆成很多小文件

规避方式：

- 每个业务模块默认先一个 `controller.rs`、一个 `service.rs`
- 真正变大再继续细分

### 风险 4：为了“纯粹架构”重新发明复杂抽象

规避方式：

- 不先建抽象 service trait 工厂
- 不先建统一 command bus
- 不先建没有真实调用方的中间接口

## 完成定义

当以下条件同时满足时，可认为本轮 Spring Boot 风格收敛完成：

1. 顶层存在清晰的 `http / security / auth / users / rbac / persistence`
2. `identity` 不再是主要业务总包
3. 前置安全与具体业务模块已经分开
4. 新业务模块默认采用 `controller + service`
5. 全局不再继续扩张 `handlers / commands / queries` 结构
6. 既有 API 行为、配置语义和网络边界保持不变
7. `docs/code-map.md` 与相关文档已同步更新

## 一句话总结

> **下一轮不是推翻 `http/` 和 `CoreState`，而是在它们之上，把当前 `identity` 继续收敛为更直白的 `security + auth + users + rbac`，并让后续业务模块统一采用更接近 Spring Boot 的 `controller + service` 结构。**
