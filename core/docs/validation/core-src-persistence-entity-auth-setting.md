# `core/src/persistence/entity/auth_setting.rs`

本文件映射 `auth_settings` 数据库实体。该实体不作为 HTTP 请求体直接接收。

## `Model`

| 字段           | 限制来源                                                                   |
|--------------|------------------------------------------------------------------------|
| `key`        | 数据库主键；当前默认只写入 `access_token_ttl_seconds` 和 `refresh_token_ttl_seconds` |
| `value`      | 数据库 `NOT NULL`；启动时按具体键解析为无符号秒数                                         |
| `updated_at` | 数据库 UTC 文本时间                                                           |

refresh token 轮换是固定安全策略，不是 `auth_settings` 配置项。
