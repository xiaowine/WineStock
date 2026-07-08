# JWT access token 与 refresh token 实现笔记

本文档记录当前鉴权 token 机制。授权模型以 [`../rbac-permission-model.md`](../rbac-permission-model.md) 为准。

## 当前行为

- `POST /api/auth/register` 在数据库没有用户时免鉴权并创建首个用户；首个用户直接获得全部内置权限。
- 已有用户后，注册新用户必须由当前拥有 `user.register` 权限的用户调用。
- 启动时先补齐内置权限定义，再初始化 JWT signing key；权限 bootstrap 不创建用户，也不写用户权限分配。
- 管理类授权以数据库当前权限为准，不只信任 JWT 中的权限快照。
- 用户状态为 `disabled` 时，access token 和 refresh token 都会被拒绝。

## 响应形状

登录和刷新成功返回：

```json
{
  "access_token": "jwt",
  "refresh_token": "opaque-token",
  "expires_in": 900,
  "user": {
    "id": "1",
    "username": "admin",
    "permissions": ["stock.read", "user.read"]
  }
}
```

`GET /api/auth/me` 返回同样的用户摘要：

```json
{
  "id": "1",
  "username": "admin",
  "permissions": ["stock.read", "user.read"]
}
```

响应体和 JWT claims 都不包含 `roles` 字段。

## Access Token

JWT access token 用 HS256 签名。签名密钥保存在数据库 `auth_signing_keys` 表中，当前只允许一个 active key。

Claims：

```text
sub: user id
jti: access token id
iat: issued-at timestamp
exp: expiry timestamp
permissions: permission list snapshot
```

JWT 中的权限是签发时快照。需要当前授权状态的接口必须在 route layer 重新读取数据库权限。

## Refresh Token

Refresh token 是高强度随机 opaque 字符串，服务端只保存 SHA-256 哈希。

`auth_refresh_tokens` 记录：

- `user_id`
- `token_hash`
- `device_name`
- `client_kind`
- `app_version`
- `refresh_token_version`
- `expires_at`
- `last_used_at`
- `revoked_at`
- `replaced_by_token_id`

刷新时执行轮换：旧 token 被新 token 替换，过期、吊销或复用的 token 会被拒绝。

## 路由

| 接口 | 鉴权 |
| --- | --- |
| `POST /api/auth/register` | 首个用户免鉴权；之后要求 `user.register` |
| `POST /api/auth/login` | 用户名密码 |
| `POST /api/auth/refresh` | refresh token |
| `POST /api/auth/logout` | refresh token |
| `GET /api/auth/me` | bearer access token |
| `POST /api/auth/me/password` | bearer access token + 当前密码 |

## 验证重点

- 首个用户无需 token 且直接获得全部内置权限。
- 后续注册必须拥有 `user.register`。
- 登录和 refresh 响应不包含 `roles`。
- 撤销数据库权限后，旧 access token 不能继续访问对应管理接口。
- 停用用户后，该用户 access token 和 refresh token 都失效。
