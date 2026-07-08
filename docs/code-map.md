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
- `docs/`：架构、网络、平台、项目结构、检查清单、数据库结构、实体限制文档、实现笔记和本代码地图。
- `docs/user-management-api.md`：当前用户管理和权限只读接口文档。
- `docs/rbac-permission-model.md`：当前用户直接权限模型、初始化行为和业务授权规则。
- `docs/validation/`：按实体所在源码文件归档的字段限制、校验入口和数据库约束说明。
- `docs/implementation-notes/README.md`：实现笔记目录说明。
- `docs/implementation-notes/core-axum-structure-refactor-plan.md`：面向后续 API 扩展的 `core\src` 领域切片重整方案。
- `docs/implementation-notes/core-spring-boot-style-refactor-plan.md`：后续把 `core\src` 从 `identity` 结构继续收敛为 `http / security / auth / users / rbac` 的实施方案。
- `docs/implementation-notes/direct-user-permissions-plan.md`：将角色间接授权收敛为用户直接权限分配的实施方案。
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
`core` 当前已按“全局 HTTP 外壳”“security 前置层”“auth 会话认证业务”“users 用户业务”“stock 库存业务”和持久化层拆分测试文件，并通过 `core/src/tests/support.rs` 复用测试搭建逻辑。
当前测试文件：`core/src/tests/support.rs`、`core/src/tests/bootstrap.rs`、`core/src/tests/http_openapi.rs`、`core/src/tests/security_authorization.rs`、`core/src/tests/auth_login.rs`、`core/src/tests/auth_refresh.rs`、`core/src/tests/auth_logout.rs`、`core/src/tests/users_register.rs`、`core/src/tests/users_me.rs`、`core/src/tests/users_management.rs`、`core/src/tests/stock_items.rs`、`core/src/tests/stock_templates.rs`、`core/src/tests/stock_inbound.rs`、`core/src/tests/stock_outbound.rs`、`core/src/tests/stock_dashboard.rs`、`core/src/tests/stock_substitutes.rs`、`core/src/tests/stock_events.rs`、`core/src/tests/persistence_connection.rs`、`core/src/tests/persistence_repository.rs`、`core/src/tests/server.rs`、`server/src/tests/lib.rs`、`server/src/tests/config.rs` 和 `shared/src/tests/lib.rs`。

## `shared`

用途：平台无关配置和契约。

- `shared/src/lib.rs`
  - 作为 `shared` crate 的薄入口，只声明 `auth`、`config`、`error` 和 `validation` 模块，并重新导出公共契约类型。
  - 不直接承载 DTO、配置实体或校验函数实现。

- `shared/src/auth.rs`
  - 定义鉴权 HTTP DTO：`AuthRegisterRequest`、`AuthLoginRequest`、`AuthRefreshRequest`、`AuthLogoutRequest`、`AuthUserResponse` 和 `AuthTokenResponse`。
  - `AuthUserResponse` 返回当前权限列表和 `password_change_required`，供客户端在临时密码登录后进入强制改密流程。
  - 定义 `AuthClientKind`，登录请求的客户端类型只允许 `desktop` 和 `android`。
  - 使用 `garde` 内置 `length`、`range`、`inner` 和项目自定义 trim/code 规则定义静态字段约束。

- `shared/src/config.rs`
  - 定义 `AppConfig`、`ServerConfig`、`StorageConfig` 和 `RuntimeMode`。
  - 使用 `garde` 内置 `dive`、`skip`、`length`、`range`、`ip` 和远端 URL 自定义规则定义 JSON 启动配置约束。
  - 提供启动配置的 JSON 解析和序列化辅助函数。
  - `AppConfig::from_json_str()` 会在 JSON 反序列化后执行 `garde` 校验，字段值不满足约束时返回 `ConfigParseError::Validation`。
  - 提供运行模式辅助判断，例如本地服务和远端服务检查。
  - `ServerConfig` 不包含单独的 `enabled` 开关；是否使用本地服务由 `RuntimeMode` 决定。

- `shared/src/error.rs`
  - 定义 `ConfigParseError`，区分 JSON 结构错误和 `garde` 字段约束错误。

- `shared/src/validation.rs`
  - 定义 DTO 和配置复用的 `garde` 自定义校验函数。
  - 只保留内置规则无法直接表达的项目语义，例如 trim 后非空、可选短标签、权限代码格式和空字符串远端 URL。

`shared` 不能依赖 `core`、Axum、平台 shell 代码、WebView 代码或前端构建产物。

## `core`

用途：供各平台 shell 复用的共享 Axum 服务库。

- `core/src/lib.rs`
  - 声明内部模块：`auth`、`bootstrap`、`http`、`persistence`、`rbac`、`security`、`server`、`state`、`stock` 和 `users`。
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
  - `router.rs` 负责组装 Swagger/OpenAPI 和业务模块 router；本地服务模式下把 `CoreState` 注入 Router，并 merge `auth`、`stock` 与 `users` 模块路由。
  - `validation.rs` 定义 `ValidatedJson<T>`，在业务 handler 之前完成 JSON 解析和 `garde` 静态字段校验；校验失败统一返回 `400 invalid_request`。

- `core/src/bootstrap.rs`
  - 定义 `CoreBootstrap` 和 `LocalServiceBootstrap`。
  - 异步实现 `bootstrap_from_config()`。
  - 仅在共享配置启用本地服务时打开本地存储，执行 migration 后先初始化顶层 `rbac` 模块的内置 RBAC，再补齐 `stock` 内置库存模板，最后初始化 `auth` 模块的鉴权设置和 JWT signing key。
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
  - 强制改密用户只允许访问 `/api/auth/me` 和 `/api/auth/me/password`；其它已鉴权接口在中间件返回 `password_change_required`。
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
  - 用户业务模块，承载注册、当前用户、当前用户修改自己密码和后续用户管理能力。
  - `mod.rs` 负责 `/api/auth/register`、`/api/auth/me`、`/api/auth/me/password`、`/api/users`、`/api/users/{id}`、`/api/users/{id}/status`、`/api/users/{id}/permissions`、`/api/users/{id}/password` 与 `/api/permissions` 的路由注册，并通过链式授权声明挂载首个用户免鉴权、已有用户注册权限、已登录校验、用户读/状态/权限更新/权限定义只读和 `user.password.reset` 重置密码权限。
  - `controller.rs` 提供注册、当前用户、当前用户修改自己密码、用户管理和权限只读接口的 HTTP 入口、DTO 和 utoipa 标注。
  - `service.rs` 处理用户注册、事务内首个用户直接分配全部内置权限、当前用户快照读取、当前用户修改自己密码、用户管理分页、账号启停、用户权限整体替换、管理员设置临时密码、最后 active 权限管理员保护、审计事件写入和响应组装；自助改密会清除强制改密标记。
  - `permissions.rs` 定义 `user.register`、`user.read`、`user.status.update`、`user.permissions.update`、`user.permission.read` 和 `user.password.reset` 等用户域稳定权限代码。

- `core/src/stock/`
  - 库存业务模块，承载物品 CRUD 和后续模板、出入库、看板、替代料、审计事件能力。
  - `mod.rs` 以 `/api` 作为库存业务 base path，负责 `items`、`templates`、`inbound`、`outbound`、`dashboard`、`events` 及其子路径的路由注册，并通过链式授权声明挂载 `stock.read`、`stock.item.manage`、`stock.template.manage`、`stock.inbound.create`、`stock.inbound.approve`、`stock.outbound.create`、`stock.outbound.approve`、`stock.substitute.manage` 与 `audit.read` 权限。
  - `controller.rs` 是库存 HTTP 控制器入口，声明并重新导出 `controller/` 下的业务子模块，保持 `stock::controller::*` 的内部访问面稳定。
  - `controller/templates.rs` 定义包含 `url` 链接字段的模板字段类型、模板 DTO、模板请求/响应和模板 Axum handler。
  - `controller/items.rs` 定义库存物品 DTO、分页查询参数、物品请求/响应和物品 Axum handler。
  - `controller/inbound.rs` 定义入库单 DTO、分页查询参数、入库请求/响应和入库 Axum handler。
  - `controller/outbound.rs` 定义出库单 DTO、分页查询参数、出库请求/响应和出库 Axum handler。
  - `controller/dashboard.rs` 定义库存看板总览、趋势查询参数、趋势响应和看板 Axum handler。
  - `controller/substitutes.rs` 定义替代料绑定请求、替代料响应和替代料 Axum handler。
  - `controller/events.rs` 定义事件日志查询参数、事件日志响应和审计事件 Axum handler。
  - `controller/common.rs` 定义多个库存 HTTP 子模块共享的单据状态枚举和正数校验函数。
  - `bootstrap.rs` 定义 `元器件`、`3D打印耗材` 和 `通用` 三个内置库存模板的启动补齐逻辑；补齐只按同名记录缺失时创建，不覆盖用户修改，也不恢复用户软删除的模板。
  - `service.rs` 是库存业务服务入口，声明并重新导出 `service/` 下的业务子模块，保持 `stock::service::*` 的内部访问面稳定。
  - `service/templates.rs` 处理模板 CRUD/copy、模板名称冲突检查、模板字段数量/唯一性/options/default 组合校验和模板写库输入组装。
  - `service/items.rs` 处理物品创建、分页、详情、更新、软删除和 SKU 冲突检查。
  - `service/inbound.rs` 处理入库创建、列表、详情、审批、拒绝和审批前模板扩展属性校验。
  - `service/outbound.rs` 处理出库创建、列表、详情、审批、拒绝和库存不足错误映射。
  - `service/dashboard.rs` 处理库存看板总览和趋势只读查询，并持有趋势天数与呆滞料阈值等看板服务常量。
  - `service/substitutes.rs` 处理替代料整体替换、查询、解绑和替代料自引用/重复/循环绑定错误映射。
  - `service/events.rs` 处理事件日志分页、筛选条件归一化和响应分页组装。
  - `service/error.rs` 定义 `StockApiError`，集中库存 HTTP 错误响应映射和 repository 自定义错误收敛。
  - `service/pagination.rs` 定义库存分页默认值、`PaginatedResponse<T>` 和总页数计算。
  - `service/response.rs` 负责把 repository 记录投影为库存 HTTP DTO，不执行数据库查询。
  - `service/validation.rs` 负责库存服务层复用的文本、数值、ID、options JSON 和扩展属性 JSON 归一化。
  - `permissions.rs` 定义 `stock.read`、`stock.write`、`stock.item.manage`、模板、出入库、替代料和 `audit.read` 等稳定权限代码。

- `core/src/rbac/`
  - 授权模型模块，承载内置权限定义和启动补齐逻辑。
  - `bootstrap.rs` 定义内置用户、库存和审计权限；启动时只补齐权限定义，不创建用户，不补齐角色或角色权限关系，也不覆盖已有权限文本。
  - 管理类授权由 `security/middleware.rs` 在校验 bearer token 后读取数据库当前权限，避免只信任过期前的 JWT 权限快照。
  - 注册接口的特殊鉴权由 `users/mod.rs` 在路由装配阶段表达：数据库没有用户时允许免鉴权进入；`users/service.rs` 会在同一事务内重新判断首个用户条件并直接分配全部内置权限，已有用户后必须由当前拥有 `user.register` 权限的 bearer token 调用。

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
  - 首版 migration 创建 `auth_users`、`auth_permissions`、`auth_user_permission_assignments`、`auth_settings`、`auth_signing_keys`、`auth_refresh_tokens`、`storage_file_objects`、`stock_templates`、`stock_template_fields`、`stock_items`、`stock_inbound_orders`、`stock_inbound_order_items`、`stock_outbound_orders`、`stock_outbound_order_items`、`stock_batches`、`stock_movements`、`stock_substitutes` 和 `audit_events`；`auth_users.password_change_required` 使用 SQLite 0/1 布尔值保存临时密码强制改密状态。
  - 为 refresh token hash、文件 hash、文件 owner/created_at、active signing key、未删除物品 SKU、未删除模板名称、FIFO 批次查询和审计查询建立索引或约束。
  - `auth_refresh_tokens` 强制保存登录设备名称、客户端类型、App 版本号和 refresh token 格式版本；客户端类型只允许桌面端或 Android 端稳定代码。

- `core/src/persistence/entity/`
  - 放置 SeaORM Entity、Model 和 ActiveModel。
  - `auth_setting.rs`、`auth_signing_key.rs`、`refresh_token.rs` 和 `user.rs` 分别映射鉴权设置、签名密钥、refresh token 和用户表。
  - `file_object.rs` 仍保存文件元数据实体。
  - `stock_item.rs` 映射库存物品基础资料表，软删除、SKU 唯一性和库存批次关系由数据库约束与仓储层共同维护。
  - `stock_template.rs` 和 `stock_template_field.rs` 分别映射库存模板基础资料和模板字段定义。

- `core/src/persistence/repository/`
  - 放置业务语义 repository。
  - `auth_repo.rs`、`audit_repo.rs`、`user_repo.rs`、`rbac_repo.rs`、`refresh_token_repo.rs` 和 `stock_repo.rs` 分别承载 auth/audit/users/rbac/refresh token/stock 的仓储能力。
  - `time.rs` 提供仓储层共用的 SQLite UTC 时间生成工具，避免具体业务仓储各自拼接时间查询。
  - `validation.rs` 提供 repository 写库输入的 `garde` 校验入口和少量内部自定义规则。
  - `AuthRepository` 支撑鉴权默认设置、active signing key 和首次管理员判断。
  - `AuditRepository` 支撑跨业务审计事件写入，调用方必须传入脱敏详情。
  - `UserRepository` 支撑用户创建、按 ID/用户名查找、分页筛选、状态更新、密码哈希更新和强制改密标记更新。
  - `RbacRepository` 支撑权限定义补齐、权限列表、用户权限查询、用户权限分配、用户权限整体替换、权限代码解析和 active 权限管理员保护查询。
  - `RefreshTokenRepository` 支撑 refresh token 创建、查询、吊销、按用户吊销 active token 和事务内轮换。
  - `file_object.rs` 中的 `FileObjectRepository` 只写入和查询文件元数据，文件内容仍归 `files/` 目录。
  - `StockRepository` 支撑库存物品创建、分页查询、详情查询、SKU 冲突检查、更新、软删除、模板 CRUD/copy、模板字段整体替换、模板名称存在性查询、模板引用检查、入库单创建/列表/详情/审批/拒绝、入库审批批次生成、出库单创建/列表/详情/审批/拒绝、指定批次或 FIFO 扣减、库存流水和审计事件写入、看板总览与趋势聚合查询、替代料整体替换/查询/解绑、循环绑定检测和事件日志分页筛选；handler 不直接拼接 `stock_*` 表结构。

- `docs/database-schema.md`
  - 记录当前 SQLite 业务表命名、职责、RBAC 链路和系统表边界。
  - 说明业务表的 `auth_`、`storage_` 前缀，避免把 SQLite 或 SeaORM 系统表误读为业务表。

- `docs/validation/`
  - `README.md` 说明限制文档的命名规则和约束来源。
  - `shared-src-auth.md` 记录鉴权 HTTP DTO 和响应实体的 `garde`/Serde 限制。
  - `shared-src-config.md` 记录启动配置和运行模式的 `garde`/Serde 限制。
  - `shared-src-error.md` 记录共享配置解析错误。
  - `shared-src-validation.md` 记录共享自定义校验函数的使用边界。
  - `core-src-persistence-entity-*.md` 记录当前 SeaORM 数据库实体的字段约束来源。
  - `core-src-persistence-repository-*.md` 记录当前 repository 写库输入结构的限制和事务边界。

- `docs/rbac-permission-model.md`
  - 记录当前用户直接权限模型的正式规则。
  - 明确业务授权统一判断权限代码，不判断角色代码。
  - 记录内置权限、用户权限分配关系、启动补齐顺序和新增受保护能力的流程。

## `server`

用途：正式无头服务端平台 shell。

- `server/src/main.rs`
  - 二进制入口。
  - 调用 `winestock_server::run()`。
  - 打印启动错误及其 source 链。

- `server/src/lib.rs`
  - 编排服务端生命周期。
  - 使用固定配置路径加载配置、校验 server 运行模式、准备存储目录、启动 `core`、绑定 Axum、打印访问 URL，并等待 Ctrl+C 关闭。
  - 打印 OpenAPI JSON 和 Swagger UI 的辅助 URL。
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

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `POST /api/auth/me/password`
- `GET /api/users`
- `GET /api/users/{id}`
- `PATCH /api/users/{id}/status`
- `PUT /api/users/{id}/permissions`
- `POST /api/users/{id}/password`
- `GET /api/permissions`
- `POST /api/templates`
- `GET /api/templates`
- `GET /api/templates/{id}`
- `PUT /api/templates/{id}`
- `DELETE /api/templates/{id}`
- `POST /api/templates/{id}/copy`
- `POST /api/items`
- `GET /api/items`
- `GET /api/items/{id}`
- `PUT /api/items/{id}`
- `DELETE /api/items/{id}`
- `POST /api/items/{id}/substitutes`
- `GET /api/items/{id}/substitutes`
- `DELETE /api/items/{id}/substitutes/{substitute_id}`
- `POST /api/inbound`
- `GET /api/inbound`
- `GET /api/inbound/{id}`
- `POST /api/inbound/{id}/approve`
- `POST /api/inbound/{id}/reject`
- `POST /api/outbound`
- `GET /api/outbound`
- `GET /api/outbound/{id}`
- `POST /api/outbound/{id}/approve`
- `POST /api/outbound/{id}/reject`
- `GET /api/dashboard/overview`
- `GET /api/dashboard/trends`
- `GET /api/events`
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

server shell 会读取或创建可执行文件同目录下的 `data/config.json`，基于共享配置启动 Axum，打印真实访问 URL，并暴露 OpenAPI JSON 和 Swagger UI 端点。
