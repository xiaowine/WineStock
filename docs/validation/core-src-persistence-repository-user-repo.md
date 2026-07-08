# `core/src/persistence/repository/user_repo.rs`

本文件定义用户写库输入和用户 repository。

## `CreateUser`

该实体由服务端业务层构造，不作为 HTTP 请求体直接接收。HTTP 注册输入限制见 `shared-src-auth.md` 中的 `AuthRegisterRequest`。

校验入口：`UserRepository::create_user()` 写库前调用 `validate_repository_input()`。

| 字段              | 限制                                                                 |
|-----------------|--------------------------------------------------------------------|
| `username`      | `garde length(min = 1, max = 64)`；trim 后非空；数据库唯一                   |
| `password_hash` | `garde length(min = 1, max = 512)`；trim 后非空；由服务端 Argon2 生成；不得是明文密码 |
| `display_name`  | 可空；存在时 trim 后非空，最大 64 字节；当前注册流程写入 `None`                           |

## `UserRepository`

写库约束：

- `create_user()` 只创建 `active` 用户。
- `list_users()` 支持按用户名/展示名模糊搜索和状态筛选；分页参数由服务层归一化。
- `update_status()` 只写入服务层已经校验过的 `active` 或 `disabled`。
- `update_password_hash()` 只接收服务端生成的 Argon2 哈希，不接收明文密码。
- `created_at` 和 `updated_at` 使用 SQLite UTC 时间生成。
- 用户名唯一性最终由 `auth_users.username` 唯一约束保证。
