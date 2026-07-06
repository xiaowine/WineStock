# 数据库结构

本文档记录当前 SQLite schema 的业务表命名、职责和系统表边界。
业务实现以 `core/src/persistence/migration/` 中的 SeaORM migration 为准；本文档用于帮助阅读数据库文件时快速区分表的所有权。

## 命名规则

业务表使用领域前缀，避免和 SQLite、SeaORM 或平台权限概念混淆：

- `auth_`：账号、角色、权限、令牌和鉴权内部状态。
- `storage_`：服务端可查询的存储元数据。

不要把 `seaql_migrations`、`sqlite_master` 或 `sqlite_sequence` 当成 WineStock 业务表。

## 业务表

### `auth_users`

账号基础表。保存登录用户名、密码哈希、展示名、账号状态和创建/更新时间。

重要字段：

- `username`：登录用户名，数据库内唯一。
- `password_hash`：密码哈希，不保存明文密码。
- `status`：账号状态，当前允许 `active` 或 `disabled`。

### `auth_roles`

角色定义表。保存角色代码、名称和说明。

重要字段：

- `code`：稳定角色代码，例如后续可用于 `admin` 或 `staff`。
- `name`：面向管理界面的角色名称。

### `auth_user_role_assignments`

用户与角色的分配表。它不是角色定义，而是记录“哪个用户拥有哪些角色”。

主键：

- `(user_id, role_id)`：同一用户不能重复分配同一角色。

### `auth_permissions`

权限定义表。保存系统可识别的权限代码和说明。

重要字段：

- `code`：稳定权限代码，例如后续可用于 `wine.read` 或 `user.manage`。
- `description`：权限说明。

### `auth_role_permission_assignments`

角色与权限的分配表。它不是权限定义，而是记录“哪个角色拥有哪些权限”。

主键：

- `(role_id, permission_id)`：同一角色不能重复分配同一权限。

### `auth_settings`

数据库托管的鉴权策略表。JSON 启动配置不保存 token TTL、refresh token 轮换等安全相关运行时策略。

当前默认设置：

- `access_token_ttl_seconds`
- `refresh_token_ttl_seconds`
- `refresh_token_rotation`

### `auth_signing_keys`

JWT access token 签名密钥表。保存系统生成的签名密钥材料和生命周期状态。

重要字段：

- `key_id`：JWT header 中的 `kid`。
- `algorithm`：签名算法，当前默认 `HS256`。
- `key_material`：签名密钥材料，不能写入日志或普通 API 响应。
- `status`：当前允许 `active` 或 `retired`。

约束：

- SQLite 局部唯一索引保证同一时间最多一条 `active` 密钥。

### `auth_refresh_tokens`

刷新令牌表。只保存 refresh token 哈希和设备元数据，不保存明文令牌。

重要字段：

- `user_id`：所属账号。
- `token_hash`：刷新令牌哈希，数据库内唯一。
- `expires_at`：过期时间。
- `revoked_at`：吊销时间，为空表示当前未吊销。

### `storage_file_objects`

文件元数据表。SQLite 只保存可查询、可校验的文件元数据；文件二进制内容由 `files/` 目录保存。

重要字段：

- `sha256`：文件内容摘要。
- `storage_path`：文件在 `files/` 目录下的相对路径。
- `owner_user_id`：文件所有者账号；用户删除后允许置空。

## 系统表

### `seaql_migrations`

SeaORM migration 版本记录表。用于记录哪些 migration 已经执行过，防止重复迁移。

### `sqlite_master`

SQLite 系统结构表。记录数据库中的表、索引、视图和触发器等对象。

### `sqlite_sequence`

SQLite 自增序列表。存在 `AUTOINCREMENT` 主键时，SQLite 用它记录自增值。

## 权限链路

用户权限通过 RBAC 链路计算：

```text
auth_users
  -> auth_user_role_assignments
  -> auth_roles
  -> auth_role_permission_assignments
  -> auth_permissions
```

repository 对外只暴露业务语义，例如按用户查询权限代码；handler 不直接依赖这些表名。
