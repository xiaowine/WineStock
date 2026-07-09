# `core/src/persistence/repository/refresh_token_repo.rs`

本文件定义 refresh token 写库输入和 refresh token repository。

## `CreateRefreshToken`

该实体由服务端鉴权业务构造，不作为 HTTP 请求体直接接收。HTTP refresh/logout 请求只接收 refresh token 明文，限制见
`core-src-auth-contract.md`。

校验入口：`RefreshTokenRepository::create()` 和 `rotate()` 写库前调用 `validate_repository_input()`。

| 字段            | 限制                                                                               |
|---------------|----------------------------------------------------------------------------------|
| `user_id`     | `garde range(min = 1)`；必须指向已存在用户；数据库外键约束保护                                       |
| `token_hash`  | `garde length(min = 1, max = 128)`；trim 后非空；服务端从明文 refresh token 计算；数据库唯一；明文不得入库 |
| `device_name` | `garde length(min = 1, max = 64)`；trim 后非空；来自登录请求                                      |
| `client_kind` | `garde length(min = 1, max = 32)`；trim 后非空；来自 `AuthClientKind` 稳定代码                    |
| `app_version` | `garde length(min = 1, max = 64)`；trim 后非空；来自登录请求中的 `version`                       |
| `refresh_token_version` | `garde length(min = 1, max = 32)`；trim 后非空；由服务端当前 refresh token 生成规则决定       |
| `expires_at`  | `garde length(min = 1, max = 64)`；trim 后非空；服务端按数据库鉴权 TTL 生成的 UTC 文本时间            |

## `RefreshTokenRepository`

写库约束：

- `create()` 只保存 token hash，不保存明文 refresh token。
- `rotate()` 在同一事务中创建新 token、吊销旧 token、记录旧 token 最近使用时间。
- `revoke()` 在事务中吊销 active token。
