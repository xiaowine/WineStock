# `core/src/persistence/entity/refresh_token.rs`

本文件映射 `auth_refresh_tokens` 数据库实体。该实体不作为 HTTP 请求体直接接收。

## `Model`

| 字段                     | 限制来源                                       |
|------------------------|--------------------------------------------|
| `id`                   | SQLite `INTEGER PRIMARY KEY AUTOINCREMENT` |
| `user_id`              | 数据库 `NOT NULL` 外键，删除用户时级联删除                |
| `token_hash`           | 数据库 `NOT NULL UNIQUE`；明文 refresh token 不入库 |
| `device_name`          | 数据库 `NOT NULL`；登录请求要求 trim 后非空、最大 64 字节      |
| `client_kind`          | 数据库 `NOT NULL`；只允许 `desktop` 或 `android`           |
| `app_version`          | 数据库 `NOT NULL`；登录请求要求 trim 后非空、最大 64 字节      |
| `refresh_token_version` | 数据库 `NOT NULL`；由服务端当前 refresh token 生成规则决定     |
| `created_at`           | 数据库 UTC 文本时间                               |
| `expires_at`           | 数据库 `NOT NULL`；由服务端按 TTL 生成                |
| `last_used_at`         | 可空；轮换时在事务内更新                               |
| `revoked_at`           | 可空；非空表示已吊销                                 |
| `replaced_by_token_id` | 可空；轮换后指向替代令牌                               |

refresh token 查询、吊销和轮换由 repository 事务保护。
