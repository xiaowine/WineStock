# `shared/src/auth.rs`

本文件定义平台无关鉴权 HTTP DTO。

## `AuthLoginRequest`

校验入口：`core::http::ValidatedJson<AuthLoginRequest>`。

| 字段            | 限制                                          |
|---------------|---------------------------------------------|
| `username`    | `garde length(min = 1, max = 64)`；trim 后非空  |
| `password`    | `garde length(min = 1, max = 256)`；trim 后非空 |
| `device_name` | `garde length(min = 1, max = 64)`；trim 后非空   |
| `client_kind` | enum 值由 Serde 限制，只允许 `desktop` 或 `android` |
| `version`     | `garde length(min = 1, max = 64)`；trim 后非空   |

## `AuthClientKind`

该实体是 enum，允许值由 Serde 反序列化限制。

JSON 允许值：

- `desktop`
- `android`

## `AuthRegisterRequest`

校验入口：`core::http::ValidatedJson<AuthRegisterRequest>`。

| 字段         | 限制                                          |
|------------|---------------------------------------------|
| `username` | `garde length(min = 1, max = 64)`；trim 后非空  |
| `password` | `garde length(min = 1, max = 256)`；trim 后非空 |

数据库状态限制：用户名唯一性、首个用户和权限规则在 `users/service.rs` 的事务内校验。

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

## `AuthTokenResponse`

该实体是服务端响应体，不作为请求体接收。

| 字段              | 限制                                           |
|-----------------|----------------------------------------------|
| `access_token`  | `garde length(min = 1, max = 8192)`；trim 后非空 |
| `refresh_token` | `garde length(min = 1, max = 512)`；trim 后非空  |
| `expires_in`    | `garde range(min = 1)`                       |
| `user`          | `garde dive` 递归校验 `AuthUserResponse`         |
