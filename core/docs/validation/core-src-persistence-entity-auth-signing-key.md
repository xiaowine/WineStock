# `core/src/persistence/entity/auth_signing_key.rs`

本文件映射 `auth_signing_keys` 数据库实体。该实体不作为 HTTP 请求体直接接收。

## `Model`

| 字段             | 限制来源                                          |
|----------------|-----------------------------------------------|
| `id`           | SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`    |
| `key_id`       | 数据库 `NOT NULL UNIQUE`；由服务端随机生成                |
| `algorithm`    | 数据库 `NOT NULL`；当前启动逻辑写入 `HS256`               |
| `key_material` | 数据库 `NOT NULL`；由服务端随机生成，不输出到普通 API            |
| `status`       | 数据库 `CHECK (status IN ('active', 'retired'))` |
| `created_at`   | 数据库 UTC 文本时间                                  |
| `activated_at` | active 密钥必须非空，由数据库 CHECK 约束保护                 |
| `retired_at`   | retired 密钥必须非空，由数据库 CHECK 约束保护                |

唯一索引 `idx_auth_signing_keys_single_active` 保证同一时间最多一条 active 密钥。
