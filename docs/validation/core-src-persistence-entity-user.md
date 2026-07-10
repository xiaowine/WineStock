# `core/src/persistence/entity/user.rs`

本文件映射 `auth_users` 数据库实体。该实体不作为 HTTP 请求体直接接收。

## `Model`

| 字段              | 限制来源                                                |
|-----------------|-----------------------------------------------------|
| `id`            | SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`          |
| `username`      | 数据库 `NOT NULL UNIQUE`；请求 DTO 额外限制 trim 后非空、最大 64 字节 |
| `password_hash` | 数据库 `NOT NULL`；由服务端 Argon2 哈希生成，不接收明文入库             |
| `status`        | 数据库 `CHECK (status IN ('active', 'disabled'))`      |
| `password_change_required` | 数据库 `NOT NULL DEFAULT 0` 且只允许 0/1；管理员设置临时密码后置为 true |
| `created_at`    | 数据库 UTC 文本时间；repository 写入统一时间                      |
| `updated_at`    | 数据库 UTC 文本时间；repository 写入统一时间                      |
