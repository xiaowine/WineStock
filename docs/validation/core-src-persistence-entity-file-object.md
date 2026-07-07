# `core/src/persistence/entity/file_object.rs`

本文件映射 `storage_file_objects` 数据库实体。该实体不作为 HTTP 请求体直接接收。

## `Model`

| 字段              | 限制来源                                       |
|-----------------|--------------------------------------------|
| `id`            | SQLite `INTEGER PRIMARY KEY AUTOINCREMENT` |
| `sha256`        | 数据库 `NOT NULL`；repository 输入当前由调用方传入       |
| `mime_type`     | 可空                                         |
| `size_bytes`    | 数据库 `CHECK (size_bytes >= 0)`              |
| `storage_path`  | 数据库 `NOT NULL`；表示 `files/` 目录下相对路径         |
| `original_name` | 可空                                         |
| `created_at`    | 数据库 UTC 文本时间                               |
| `owner_user_id` | 可空外键；用户删除后置空                               |

文件二进制内容不进入 SQLite；数据库只保存可查询和可校验的元数据。
