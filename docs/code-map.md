# 代码地图

本文档记录当前源码布局和所有权边界。
生成新代码、增加或移动模块和 crate、修改公共 API，或进行较大范围实现改动后，都要同步更新本文档。
本文档必须使用中文编写和维护。

## 当前范围

WineStock 的正式产品目标是多平台，但当前实现范围是 server/API 优先。

当前正式 Rust 工作区成员：

- `core`
- `server`
- `shared`

当前非主要脚手架：

- `frontend` 是已有 Vue/Vite 脚手架和前端源码区域。它不由 Axum 服务，也不代表 Vue 是固定架构选择。
- `desktop` 是普通 Rust 脚手架，不是工作区成员，也不是正式 Tauri shell。
- 正式 Android shell 代码目前不存在。

## 根目录

- `AGENTS.md`：agent 的简短操作指南。
- `Cargo.toml`：Cargo 工作区成员和共享依赖版本。
- `Cargo.lock`：Rust 依赖锁文件。
- `docs/`：架构、网络、平台、项目结构、检查清单、数据库结构、实现笔记和本代码地图。
- `docs/rbac-permission-model.md`：当前 RBAC 角色/权限模型、初始化行为和业务授权规则。
- `docs/implementation-notes/README.md`：实现笔记目录说明。
- `docs/implementation-notes/core-axum-structure-refactor-plan.md`：面向后续 API 扩展的 `core\src` 领域切片重整方案。
- `docs/implementation-notes/core-spring-boot-style-refactor-plan.md`：后续把 `core\src` 从 `identity` 结构继续收敛为 `http / security / auth / users / rbac` 的实施方案。
- `docs/implementation-notes/json-config-and-db-auth-settings.md`：JSON 启动配置与数据库托管鉴权设置的边界说明。
- `docs/implementation-notes/jwt-access-refresh-token.md`：JWT access token 与 refresh token 机制实现笔记。
- `docs/implementation-notes/seaorm-sqlite-wal.md`：SeaORM、SQLite 和 WAL 存储行为实现笔记。
- `core/`：共享 Rust/Axum 服务库。
- `shared/`：平台无关配置、契约和通用类型。
- `server/`：运行共享服务的无头服务端 shell。
- `frontend/`：仅作为前端脚手架和源码区域。
- `desktop/`：非正式的普通 Rust 脚手架。

## 工作区依赖方向

允许方向：

```text
server -> core -> shared
server -> shared
```

禁止方向：

```text
shared -> core
core   -> server
core   -> desktop/android/frontend platform assets
```

## 测试布局

单元测试统一放在各 crate 的 `src/tests/` 目录中，源码文件只保留 `#[cfg(test)]`、`#[path = "..."]` 和对应测试模块声明。
测试仍作为被测模块的子模块挂载，因此可以访问本模块私有项；物理文件集中存放，避免生产代码文件夹中散落 `tests.rs`。
`core` 当前已按“全局 HTTP 外壳”“security 前置层”“auth 会话认证业务”“users 用户业务”和持久化层拆分测试文件，并通过 `core/src/tests/support.rs` 复用测试搭建逻辑。
当前测试文件：`core/src/tests/support.rs`、`core/src/tests/bootstrap.rs`、`core/src/tests/http_health.rs`、`core/src/tests/http_openapi.rs`、`core/src/tests/security_authorization.rs`、`core/src/tests/auth_login.rs`、`core/src/tests/auth_refresh.rs`、`core/src/tests/auth_logout.rs`、`core/src/tests/users_register.rs`、`core/src/tests/users_me.rs`、`core/src/tests/persistence_connection.rs`、`core/src/tests/persistence_repository.rs`、`core/src/tests/server.rs`、`server/src/tests/lib.rs`、`server/src/tests/config.rs` 和 `shared/src/tests/lib.rs`。

## `shared`

用途：平台无关配置和契约。

- `shared/src/lib.rs`
  - 定义鉴权 HTTP DTO：`AuthRegisterRequest`、`AuthLoginRequest`、`AuthRefreshRequest`、`AuthLogoutRequest`、`AuthUserResponse` 和 `AuthTokenResponse`。
  - 定义 `AppConfig`、`ServerConfig`、`StorageConfig` 和 `RuntimeMode`。
  - 提供启动配置的 JSON 解析和序列化辅助函数。
  - 提供运行模式辅助判断，例如本地服务和远端服务检查。
  - `ServerConfig` 不包含单独的 `enabled` 开关；是否使用本地服务由 `RuntimeMode` 决定。
  - 不能依赖 `core`、Axum、平台 shell 代码、WebView 代码或前端构建产物。

## `core`

用途：供各平台 shell 复用的共享 Axum 服务库。

- `core/src/lib.rs`
  - 声明内部模块：`auth`、`bootstrap`、`http`、`persistence`、`rbac`、`security`、`server`、`state` 和 `users`。
  - 重新导出 core 的公共启动入口、HTTP 构建入口、鉴权公开类型和运行时错误类型。
  - 重新导出 `RbacBootstrapError`，供平台 shell 区分内置 RBAC 初始化失败。
  - 重新导出 `winestock_shared` 为 `shared`，供调用方通过 core 入口访问共享契约。
  - 保留 `build_router()` 和 `build_router_with_local_service()` 两个稳定入口，但不直接承担 Router 细节和 OpenAPI 元信息。

- `core/src/state.rs`
  - 定义统一的 `CoreState`。
  - 把 `StorageRuntime` 和 `SecurityRuntime` 组合成全局 Axum state 根对象。
  - 避免某个领域 runtime 直接充当整个服务状态。

- `core/src/http/`
  - 作为唯一的全局 HTTP 外壳层。
  - `docs.rs` 定义 `OPENAPI_JSON_PATH`、`SWAGGER_UI_PATH`、OpenAPI 元信息和 Swagger UI 挂载。
  - `health.rs` 定义 `HealthResponse` 和 `GET /api/health`。
  - `router.rs` 负责组装健康检查、Swagger/OpenAPI 和业务模块 router；本地服务模式下把 `CoreState` 注入 Router，并 merge `auth` 与 `users` 模块路由。

- `core/src/bootstrap.rs`
  - 定义 `CoreBootstrap` 和 `LocalServiceBootstrap`。
  - 异步实现 `bootstrap_from_config()`。
  - 仅在共享配置启用本地服务时打开本地存储，执行 migration 后先初始化顶层 `rbac` 模块的内置 RBAC，再初始化 `auth` 模块的鉴权设置和 JWT signing key。
  - 对远端-only 或禁用本地服务的模式跳过存储初始化。

- `core/src/server.rs`
  - 定义 `BoundServer` 和 `ServerStartError`。
  - 基于共享 `ServerConfig` 实现 `bind_server()`。
  - 拥有按配置绑定 socket、报告端口冲突和优雅运行 Axum 的逻辑。
  - 不决定平台生命周期，也不决定面向用户的展示文本。

- `core/src/security/`
  - 全局认证与授权前置层，不属于具体业务域。
  - `current_user.rs` 定义 `CurrentUser` extractor 和 bearer token 解析。
  - `jwt.rs` 定义 `SecurityRuntime`、JWT claims 和 access token 的签发/校验逻辑。
  - `middleware.rs` 定义 Axum route layer 鉴权中间件和 `AuthorizeRouteExt` 链式路由授权声明；普通 API 可在业务模块路由注册处声明所需权限，中间件会重新读取数据库当前权限后再放行业务 handler。
  - `password.rs` 集中处理 Argon2 密码哈希与校验。
  - `token.rs` 集中处理 refresh token 的 SHA-256 哈希、高强度随机文本和 JWT 时间戳。
  - `error.rs` 定义 `security`、`auth` 和 `users` 共用的鉴权 HTTP 错误和响应映射。

- `core/src/auth/`
  - 会话认证业务模块，承载登录、refresh、logout 和 auth bootstrap。
  - `mod.rs` 负责 `/api/auth/login`、`/api/auth/refresh` 和 `/api/auth/logout` 的路由注册。
  - `controller.rs` 提供对应 HTTP 入口和 utoipa 标注。
  - `service.rs` 处理登录、refresh token 轮换、旧 token 复用检测和登出吊销逻辑。
  - `bootstrap.rs` 定义鉴权启动设置、签名密钥状态和鉴权启动结果；通过 `AuthRepository` 写入默认鉴权设置但不覆盖数据库管理的已有值，并创建或读取当前 active 访问令牌签名密钥。

- `core/src/users/`
  - 用户业务模块，承载注册、当前用户和后续用户管理能力。
  - `mod.rs` 负责 `/api/auth/register` 与 `/api/auth/me` 的路由注册，并通过链式授权声明挂载首个用户免鉴权、已有用户注册权限和已登录校验。
  - `controller.rs` 提供注册与当前用户接口的 HTTP 入口和 utoipa 标注。
  - `service.rs` 处理首个管理员分配、当前用户快照读取、用户响应组装和用户名规范化。
  - `permissions.rs` 定义 `user.register`、`user.manage` 等用户域稳定权限代码。

- `core/src/rbac/`
  - 授权模型模块，承载内置角色/权限常量和启动补齐逻辑。
  - `policy.rs` 定义稳定角色代码，以及在 `stock` 正式模块落地前暂存的库存权限常量。
  - `bootstrap.rs` 定义内置 RBAC 基础数据，包括 `admin`、`staff`、`viewer` 角色和基础用户/库存权限；启动时补齐角色、权限和角色权限关系，不创建用户，也不覆盖已有角色或权限文本。
  - 角色只作为批量授予权限的模板，不作为业务授权等级。
  - 管理类授权由 `security/middleware.rs` 在校验 bearer token 后读取数据库当前权限，避免只信任过期前的 JWT 权限快照。
  - 注册接口的特殊鉴权由 `users/mod.rs` 在路由装配阶段处理：数据库没有用户时免鉴权并把首个用户分配为 `admin`；已有用户后必须由当前拥有 `user.register` 权限的 bearer token 调用。

- `core/src/persistence/`
  - 定义 `StorageRuntime` 和存储启动错误。
  - 通过 SeaORM/SQLx 打开 SQLite，并向 core 暴露 `DatabaseConnection`。
  - 集中应用 SQLite PRAGMA 设置，例如 foreign keys、busy timeout、WAL 和 checkpoint 行为。
  - 校验平台 shell 传入的存储路径。
  - 按 `StorageConfig.auto_migrate` 执行 SeaORM migration。
  - `entity/` 和 `repository/` 使用对齐当前业务模块的直白命名，避免再保留 `identity/` 中间目录。

- `core/src/persistence/connection.rs`
  - 打开 SQLite 文件连接池。
  - 配置 WAL、foreign keys、busy timeout 和 `wal_autocheckpoint`。
  - 执行 migration 入口。

- `core/src/persistence/migration/`
  - 定义 SeaORM `Migrator`。
  - 首版 migration 创建 `auth_users`、`auth_roles`、`auth_user_role_assignments`、`auth_permissions`、`auth_role_permission_assignments`、`auth_settings`、`auth_signing_keys`、`auth_refresh_tokens` 和 `storage_file_objects`。
  - 为 refresh token hash、文件 hash、文件 owner/created_at 和 active signing key 建立索引或约束。

- `core/src/persistence/entity/`
  - 放置 SeaORM Entity、Model 和 ActiveModel。
  - `auth_setting.rs`、`auth_signing_key.rs`、`refresh_token.rs` 和 `user.rs` 分别映射鉴权设置、签名密钥、refresh token 和用户表。
  - `file_object.rs` 仍保存文件元数据实体。

- `core/src/persistence/repository/`
  - 放置业务语义 repository。
  - `auth_repo.rs`、`user_repo.rs`、`rbac_repo.rs` 和 `refresh_token_repo.rs` 分别承载 auth/users/rbac/refresh token 的仓储能力。
  - `time.rs` 提供仓储层共用的 SQLite UTC 时间生成工具，避免具体业务仓储各自拼接时间查询。
  - `AuthRepository` 支撑鉴权默认设置、active signing key 和首次管理员判断。
  - `UserRepository` 只支撑用户创建、按 ID/用户名查找。
  - `RbacRepository` 支撑角色/权限定义补齐、用户角色分配、角色权限分配、角色权限同步、角色列表和权限列表查询。
  - `RefreshTokenRepository` 支撑 refresh token 创建、查询、吊销和事务内轮换。
  - `file_object.rs` 中的 `FileObjectRepository` 只写入和查询文件元数据，文件内容仍归 `files/` 目录。

- `docs/database-schema.md`
  - 记录当前 SQLite 业务表命名、职责、RBAC 链路和系统表边界。
  - 说明业务表的 `auth_`、`storage_` 前缀，避免把 SQLite 或 SeaORM 系统表误读为业务表。

- `docs/rbac-permission-model.md`
  - 记录当前 RBAC 模型的正式规则。
  - 明确业务授权统一判断权限代码，不判断角色代码。
  - 说明角色只作为批量授予权限的模板，不作为业务授权等级。
  - 记录内置角色、内置权限、角色权限关系、启动补齐顺序和新增受保护能力的流程。

## `server`

用途：正式无头服务端平台 shell。

- `server/src/main.rs`
  - 二进制入口。
  - 调用 `winestock_server::run()`。
  - 打印启动错误及其 source 链。

- `server/src/lib.rs`
  - 编排服务端生命周期。
  - 使用固定配置路径加载配置、校验 server 运行模式、准备存储目录、启动 `core`、绑定 Axum、打印访问 URL，并等待 Ctrl+C 关闭。
  - 打印健康检查、OpenAPI JSON 和 Swagger UI 的辅助 URL。
  - 直接处理最小的绑定地址展示；绑定到所有接口时使用 loopback URL 作为本机访问地址，不把 `0.0.0.0` 展示为可打开 URL。

- `server/src/config.rs`
  - 使用当前可执行文件目录固定定位 `data/config.json`。
  - 不解析命令行配置路径参数。
  - 读取已有 JSON 配置，或在缺失时创建固定位置的 `data/config.json`。
  - 以配置文件所在目录为基准解析相对存储路径。
  - 默认生成的 `winestock.sqlite` 和 `files` 路径都位于同一个 `data` 目录下。
  - 确保 server shell 只运行本地服务模式。
  - 在 core 打开 SQLite 前创建必要的存储目录。

- `server/src/error.rs`
  - 定义 `ServerShellError`。
  - 集中管理无头 shell 的配置、存储准备、core 启动和服务启动错误。

- `server/data/`
  - 旧的开发运行时数据目录，不是固定配置位置。
  - 固定配置位置是运行时可执行文件同目录下的 `data/config.json`。

## 运行流程

```text
server/src/main.rs
  -> winestock_server::run()
  -> config::fixed_config_path()
  -> config::load_config()
  -> config::ensure_server_runtime()
  -> config::prepare_storage_dirs()
  -> winestock_core::bootstrap_from_config().await
  -> winestock_core::bind_server()
  -> BoundServer::serve_local_with_shutdown()
  -> winestock_core::build_router_with_local_service()
```

## 公共 HTTP 接口

- `GET /api/health`
- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `GET /api-docs/openapi.json`
- `/swagger-ui` 下的 Swagger UI

## 前端和桌面说明

- `frontend/package.json` 当前描述的是 Vue/Vite 脚手架。
- `frontend/src/` 包含 demo 前端源码和资源。
- Axum 不能服务 `frontend` 构建产物。
- `desktop/Cargo.toml` 和 `desktop/src/main.rs` 不是当前 Cargo 工作区的一部分。
- 不要从这些脚手架推断正式 desktop、Android 或包命名决策。

## 验证入口

当前 Rust 检查：

```text
cargo check --workspace --all-targets
cargo test --workspace
cargo build -p winestock-server
```

本地 server smoke test：

```text
cargo run -p winestock-server
```

server shell 会读取或创建可执行文件同目录下的 `data/config.json`，基于共享配置启动 Axum，打印真实访问 URL，并暴露健康检查、OpenAPI JSON 和 Swagger UI 端点。
