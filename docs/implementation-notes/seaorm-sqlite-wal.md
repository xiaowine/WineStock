# SeaORM + SQLite + WAL 实现方案

## 文档状态

本文是后续实现持久化层的方案文档，用于记录设计意图和实现方向。

除非被正式规范文档引用，本文不作为日常 agent 的强制约束。

## 目标

WineStock 的 v1 持久化层采用 `SeaORM + SQLite + WAL`。

职责边界：

- `core` 负责数据库连接、迁移、Repository 和事务。
- `shared` 只放平台无关配置、DTO、ID 类型和查询参数。
- 桌面/Android 平台壳只负责提供数据目录和数据库文件路径。

默认存储形态：

```text
data/
  winestock.sqlite
  winestock.sqlite-wal
  winestock.sqlite-shm
  files/
```

SQLite 存结构化数据、索引、鉴权数据、文件元数据；大对象文件放 `files/` 目录。

## 依赖选择

workspace 依赖中加入 SeaORM 相关依赖：

```toml
sea-orm = { version = "...", features = ["sqlx-sqlite", "runtime-tokio", "macros"] }
sea-orm-migration = { version = "...", features = ["sqlx-sqlite", "runtime-tokio"] }
```

版本号以后续实际实现时的可用稳定版本为准。

选择 SeaORM 的原因：

- async，贴合 Axum/Tokio。
- ORM 风格，减少手写 SQL。
- 支持 Entity、Model、ActiveModel、Migration。
- 支持 SQLite，并保留未来 PostgreSQL 路线。

不直接选择裸 SQLx 的原因：

- 项目希望减少业务代码中的 SQL 语句。
- SeaORM 底层已经能适配 SQLx 生态。

不在 v1 引入 libSQL remote/sync：

- v1 先解决本地可靠存储。
- 远程副本和离线同步需要额外设计冲突、权限、同步边界。

## 初始化流程

数据库初始化流程固定为：

```text
平台壳提供 StorageConfig
core 创建 DatabaseConnection
core 执行 SQLite PRAGMA
core 根据 auto_migrate 执行 migration
core 构建 AppState / CoreState
Router handlers 通过 State 使用 repository
```

`shared` 中定义平台无关存储配置：

```text
StorageConfig
  database_path
  files_dir
  auto_migrate
```

`core` 不硬编码数据库路径，不选择用户目录，不处理平台权限弹窗。

## SQLite WAL 配置

SQLite PRAGMA 默认值：

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA wal_autocheckpoint = 1000;
```

含义：

- `journal_mode = WAL`：启用 Write-Ahead Logging，提高读写并发能力。
- `foreign_keys = ON`：启用外键约束。
- `busy_timeout = 5000`：数据库短暂繁忙时等待最多 5 秒。
- `wal_autocheckpoint = 1000`：WAL 日志达到约 1000 页后自动 checkpoint。

这些 PRAGMA 必须集中在数据库初始化层执行，不能散落到业务 handler。

## Core 持久化模块

`core` 增加 persistence 模块，职责包括：

- 打开 SQLite 连接。
- 启用 WAL 和基础 PRAGMA。
- 执行 migration。
- 暴露 Repository 给 auth 和业务模块使用。
- 管理事务边界。

推荐内部结构：

```text
core/src/persistence/
  mod.rs
  connection.rs
  migration/
  entity/
  repository/
```

说明：

- `entity/` 放 SeaORM Entity、Model、ActiveModel。
- `repository/` 暴露业务语义方法。
- handler 不直接散写 ORM 查询。
- migration 和底层 SQLite 初始化允许写少量 SQL。

## 首版数据表

首版 migration 至少创建：

```text
auth_users
auth_roles
auth_user_role_assignments
auth_permissions
auth_role_permission_assignments
auth_settings
auth_signing_keys
auth_refresh_tokens
storage_file_objects
```

`auth_settings` 表用于保存鉴权策略，不放入 JSON 配置：

```text
key
value
updated_at
```

`auth_signing_keys` 表用于保存 JWT access token 签名密钥：

```text
id
key_id
algorithm
key_material
status
created_at
activated_at
retired_at
```

`auth_refresh_tokens` 表用于衔接 JWT 方案：

```text
id
user_id
token_hash
device_name
client_kind
created_at
expires_at
last_used_at
revoked_at
```

`storage_file_objects` 表只保存文件元数据：

```text
id
sha256
mime_type
size_bytes
storage_path
original_name
created_at
owner_user_id
```

大文件内容不存 SQLite BLOB，放入 `files/` 目录。

## Repository 边界

Repository 对 handler 暴露业务语义，不暴露 SeaORM 细节。

示例方法形态：

```text
UserRepository
  find_by_username
  find_by_id
  create_user
  list_user_permissions

RefreshTokenRepository
  create
  find_active_by_hash
  rotate
  revoke
  revoke_chain

FileObjectRepository
  create_metadata
  find_by_id
  find_by_sha256
```

鉴权中的 refresh token 查询和轮换必须在事务中完成。

## 搜索策略

v1 不强制启用 FTS5。

普通索引先覆盖：

- 用户名查找。
- refresh token hash 查找。
- 文件 sha256 查找。
- owner/user/filter 类查询。
- 创建时间排序。

后续需要全文搜索时，通过新 migration 增加 SQLite FTS5 虚表。

如果 FTS5 不能满足复杂相关性排序、搜索高亮或大规模文本检索，再评估 Tantivy。

## 平台边界

desktop shell：

- 使用平台数据目录生成 `database_path` 和 `files_dir`。
- 将 `StorageConfig` 传给 core。

Android shell：

- 使用 app 私有数据目录生成 `database_path` 和 `files_dir`。
- 将 `StorageConfig` 传给 core。

core：

- 只接收路径和配置。
- 不关心路径来自桌面、Android 还是 server-mode。
- 不管理平台文件权限弹窗。

## 测试场景

必须覆盖：

- 首次启动时自动创建 SQLite 文件。
- 初始化后能启用 WAL。
- `PRAGMA journal_mode` 返回 `wal`。
- `PRAGMA foreign_keys` 返回启用状态。
- migration 可重复执行，不破坏已有数据。
- 缺失数据库目录时返回清晰错误。
- refresh token 写入、查询、吊销、轮换在事务中正确执行。
- 并发读取不被普通写入长期阻塞。
- 文件元数据写入 SQLite，大文件内容写入 `files/`。
- `cargo check --workspace --all-targets` 通过。

## 当前假设

- v1 只支持 SQLite 本地文件，不支持 PostgreSQL/libSQL remote。
- v1 使用 SeaORM，不直接使用裸 SQLx repository。
- 允许在 migration、SQLite PRAGMA 初始化、FTS5 相关能力中写少量 SQL。
- 大文件不存 SQLite BLOB，存文件系统；SQLite 只存元数据。
- 默认自动迁移开启，后续生产 server-mode 如有需要再增加手动迁移开关。
- 本方案是实现方案，不自动变成项目规范，除非后续写入正式规范文档。
