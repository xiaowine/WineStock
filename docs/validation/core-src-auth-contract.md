# `core/src/auth/contract.rs`

本文件定义 core 鉴权 HTTP DTO，属于 Axum 服务 API 契约，不属于 `shared` 运行配置层。

## `AuthLoginRequest`

校验入口：`core::http::ValidatedJson<AuthLoginRequest>`。

| 字段            | 限制                                          |
|---------------|---------------------------------------------|
| `username`    | `garde length(min = 1, max = 64)`；trim 后非空  |
| `password`    | `garde length(min = 1, max = 256)`；trim 后非空 |
| `device_name` | `garde length(min = 1, max = 64)`；trim 后非空   |
| `client_kind` | enum 值由 Serde 限制，只允许 `desktop`、`android` 或 `web` |
| `version`     | `garde length(min = 1, max = 64)`；trim 后非空   |

## `AuthClientKind`

该实体是 enum，允许值由 Serde 反序列化限制。

JSON 允许值：

- `desktop`
- `android`
- `web`

## `AuthRegisterRequest`

校验入口：`core::http::ValidatedJson<AuthRegisterRequest>`。

| 字段         | 限制                                          |
|------------|---------------------------------------------|
| `username` | `garde length(min = 1, max = 64)`；trim 后非空  |
| `password` | `garde length(min = 1, max = 256)`；trim 后非空 |

数据库状态限制：用户名唯一性、首个用户和权限规则在 `users/service/register.rs` 的事务内校验。

## `AuthRefreshRequest`

校验入口：`core::http::ValidatedJson<AuthRefreshRequest>`。

| 字段              | 限制                                          |
|-----------------|---------------------------------------------|
| `refresh_token` | `garde length(min = 1, max = 512)`；trim 后非空 |

## `AuthLogoutRequest`

校验入口：`core::http::ValidatedJson<AuthLogoutRequest>`。

| 字段              | 限制                                          |
|-----------------|---------------------------------------------|
| `refresh_token` | `garde length(min = 1, max = 512)`；trim 后非空 |

## `AuthUserResponse`

该实体是服务端响应体，不作为请求体接收。

| 字段            | 限制                                                                                  |
|---------------|-------------------------------------------------------------------------------------|
| `id`          | `garde length(min = 1, max = 32)`；trim 后非空                                          |
| `username`    | `garde length(min = 1, max = 64)`；trim 后非空                                          |
| `permissions` | 每项 `garde inner(length(min = 1, max = 128))`；每项 trim 后非空；只允许小写 ASCII、数字、`.`、`-`、`_` |
| `password_change_required` | 布尔响应字段；临时密码登录后为 `true`，自助改密后恢复 `false` |

## `AuthTokenResponse`

该实体是服务端响应体，不作为请求体接收。

| 字段              | 限制                                           |
|-----------------|----------------------------------------------|
| `access_token`  | `garde length(min = 1, max = 8192)`；trim 后非空 |
| `refresh_token` | `garde length(min = 1, max = 512)`；trim 后非空  |
| `expires_in`    | `garde range(min = 1)`                       |
| `user`          | `garde dive` 递归校验 `AuthUserResponse`         |
