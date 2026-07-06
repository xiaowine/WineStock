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

## `shared`

用途：平台无关配置和契约。

- `shared/src/lib.rs`
  - 定义 `AppConfig`、`ServerConfig`、`StorageConfig` 和 `RuntimeMode`。
  - 提供启动配置的 JSON 解析和序列化辅助函数。
  - 提供运行模式辅助判断，例如本地服务和远端服务检查。
  - `ServerConfig` 不包含单独的 `enabled` 开关；是否使用本地服务由 `RuntimeMode` 决定。
  - 不能依赖 `core`、Axum、平台 shell 代码、WebView 代码或前端构建产物。

## `core`

用途：供各平台 shell 复用的共享 Axum 服务库。

- `core/src/lib.rs`
  - 声明内部模块：`auth`、`bootstrap`、`persistence`、`server`。
  - 重新导出 core 的公共启动和运行时类型。
  - 定义 `OPENAPI_JSON_PATH` 和 `SWAGGER_UI_PATH`。
  - 通过 `build_router()` 构建 Axum 路由。
  - 拥有 `GET /api/health`、OpenAPI 元信息和 Swagger UI 挂载。

- `core/src/bootstrap.rs`
  - 定义 `CoreBootstrap` 和 `LocalServiceBootstrap`。
  - 异步实现 `bootstrap_from_config()`。
  - 仅在共享配置启用本地服务时打开本地存储并初始化鉴权。
  - 对远端-only 或禁用本地服务的模式跳过存储初始化。

- `core/src/server.rs`
  - 定义 `BoundServer` 和 `ServerStartError`。
  - 基于共享 `ServerConfig` 实现 `bind_server()`。
  - 拥有按配置绑定 socket、报告端口冲突和优雅运行 Axum 的逻辑。
  - 不决定平台生命周期，也不决定面向用户的展示文本。

- `core/src/persistence/`
  - 定义 `StorageRuntime` 和存储启动错误。
  - 通过 SeaORM/SQLx 打开 SQLite，并向 core 暴露 `DatabaseConnection`。
  - 集中应用 SQLite PRAGMA 设置，例如 foreign keys、busy timeout、WAL 和 checkpoint 行为。
  - 校验平台 shell 传入的存储路径。
  - 按 `StorageConfig.auto_migrate` 执行 SeaORM migration。
  - 放置 SeaORM Entity、migration 和 repository，handler 不直接散写 ORM 查询。

- `core/src/persistence/connection.rs`
  - 打开 SQLite 文件连接池。
  - 配置 WAL、foreign keys、busy timeout 和 `wal_autocheckpoint`。
  - 执行 migration 入口。

- `core/src/persistence/migration/`
  - 定义 SeaORM `Migrator`。
  - 首版 migration 创建 `auth_users`、`auth_roles`、`auth_user_role_assignments`、`auth_permissions`、`auth_role_permission_assignments`、`auth_settings`、`auth_signing_keys`、`auth_refresh_tokens` 和 `storage_file_objects`。
  - 为 refresh token、文件 hash、文件 owner/created_at 和 active signing key 建立索引或约束。

- `core/src/persistence/entity/`
  - 放置 SeaORM Entity、Model 和 ActiveModel。
  - 当前包含 auth setting、auth signing key、user、refresh token 和 file object 实体。

- `core/src/persistence/repository/`
  - 放置业务语义 repository。
  - `AuthRepository` 支撑鉴权默认设置、active signing key 和首次管理员判断。
  - `UserRepository` 支撑用户创建、按 ID/用户名查找和权限列表查询。
  - `RefreshTokenRepository` 支撑 refresh token 创建、查询、吊销和事务内轮换。
  - `FileObjectRepository` 只写入和查询文件元数据，文件内容仍归 `files/` 目录。

- `core/src/auth.rs`
  - 定义鉴权启动设置、签名密钥状态和鉴权启动结果。
  - 通过 `AuthRepository` 写入默认鉴权设置，但不覆盖数据库管理的已有值。
  - 创建或读取当前 active 访问令牌签名密钥。
  - 判断是否仍需首次管理员初始化。

- `docs/database-schema.md`
  - 记录当前 SQLite 业务表命名、职责、RBAC 链路和系统表边界。
  - 说明业务表的 `auth_`、`storage_` 前缀，避免把 SQLite 或 SeaORM 系统表误读为业务表。

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
  -> BoundServer::serve_with_shutdown()
  -> winestock_core::build_router()
```

公共 HTTP 接口：

- `GET /api/health`
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
