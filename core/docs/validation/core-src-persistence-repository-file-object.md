# `core/src/persistence/repository/file_object.rs`

本文件定义文件元数据写库输入和文件对象 repository。

## `CreateFileObject`

该实体由服务端文件或导入流程构造，不作为 HTTP 请求体直接接收。

校验入口：`FileObjectRepository::create_metadata()` 写库前调用 `validate_repository_input()`。

| 字段              | 限制                                                                              |
|-----------------|---------------------------------------------------------------------------------|
| `sha256`        | `garde length(min = 1, max = 128)`；trim 后非空；调用方必须在写入元数据前计算；数据库建立摘要索引            |
| `mime_type`     | 可空；存在时 trim 后非空，最大 255 字节                                                       |
| `size_bytes`    | `garde range(min = 0)`；数据库 `CHECK (size_bytes >= 0)`                            |
| `storage_path`  | `garde length(min = 1, max = 4096)`；trim 后非空；数据库 `NOT NULL`；应为 `files/` 目录下相对路径 |
| `original_name` | 可空；存在时 trim 后非空，最大 255 字节                                                       |
| `owner_user_id` | 可空；存在时必须是正整数；存在时应指向用户，用户删除后置空                                                   |

## `FileObjectRepository`

写库约束：

- 只写入和查询 SQLite 文件元数据。
- 不读写文件二进制内容。
- 文件内容必须已由调用方写入 `StorageRuntime.files_dir` 对应目录。
