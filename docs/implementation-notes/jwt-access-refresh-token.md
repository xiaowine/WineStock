# JWT Access Token + Refresh Token 方案

## 文档状态

本文是 JWT access token + refresh token 鉴权功能的方案文档，并记录当前首版落地范围。

除非被正式规范文档引用，本文不作为日常 agent 的强制约束。

当前实现已落地到 `core` 和 `shared`：

- `shared` 定义注册、登录、刷新、登出和当前用户响应 DTO。
- `core` 挂载 `POST /api/auth/register`、`POST /api/auth/login`、`POST /api/auth/refresh`、`POST /api/auth/logout` 和 `GET /api/auth/me`。
- `POST /api/auth/register` 在数据库没有用户时免鉴权并自动创建首个 `admin` 用户；已有用户后必须由当前拥有 `user.register` 权限的用户调用。
- 启动时先自动补齐 `admin`、`staff`、`viewer` 内置角色和 `user.register`、`user.manage`、`stock.read`、`stock.write` 内置权限，再初始化 JWT signing key；RBAC bootstrap 不创建用户，不覆盖已有角色或权限文本。
- `core` 使用数据库中的 active HS256 signing key 签发和校验 JWT access token。
- 管理类授权以数据库当前权限为准，不只信任 JWT 中的权限快照；角色只负责批量授予权限。
- refresh token 使用高强度随机 opaque token，数据库只保存 SHA-256 哈希。
- refresh token 轮换会吊销旧 token 并记录替代 token；检测到旧 token 复用时返回 401。
- 同一用户允许多条未吊销 refresh token；一个设备轮换 token 不影响其它设备重新打开后继续刷新。

RBAC 的当前正式规则见 [`../rbac-permission-model.md`](../rbac-permission-model.md)。

## 目标

WineStock 鉴权采用正式多用户模型，使用 `JWT access token + refresh token`，面向前后端分离、桌面端、Android 端和远程客户端统一接入。

默认策略：

- access token：JWT，15 分钟有效。
- refresh token：不使用 JWT，使用高强度随机 opaque token，7 天有效。
- 刷新时轮换 refresh token，旧 refresh token 立即失效。
- 多设备登录各自持有独立 refresh token，设备 A 刷新不会吊销设备 B 的 refresh token。
- 客户端通过 JSON 接收 token，并使用 `Authorization: Bearer <access_token>` 调用受保护 API。
- refresh token 只在服务端保存哈希，不保存明文。
- `core` 负责鉴权 API、JWT 签发/校验、refresh token 生命周期和权限校验。
- `shared` 负责平台无关 DTO、启动配置结构、用户/权限契约。

## API 设计

新增认证 API：

```text
POST /api/auth/login
POST /api/auth/register
POST /api/auth/refresh
POST /api/auth/logout
GET  /api/auth/me
```

### `POST /api/auth/register`

输入：

```json
{
  "username": "admin",
  "password": "password"
}
```

行为要求：

- 当数据库还没有任何用户时，不需要 `Authorization` 请求头。
- 首个注册用户自动获得 `admin` 角色。
- 当数据库已有用户后，调用方必须提供当前拥有 `user.register` 权限的 Bearer access token。
- 注册接口保存 Argon2 密码哈希，不保存明文密码。

成功返回：

```json
{
  "id": "<user-id>",
  "username": "admin",
  "roles": ["admin"],
  "permissions": ["stock.read", "stock.write", "user.manage", "user.register"]
}
```

失败返回：

```text
400 invalid_register_request
401 invalid_access_token
403 permission_denied
409 username_taken
```

### `POST /api/auth/login`

输入：

```json
{
  "username": "admin",
  "password": "password"
}
```

成功返回：

```json
{
  "access_token": "<jwt>",
  "refresh_token": "<opaque-token>",
  "expires_in": 900,
  "user": {
    "id": "<user-id>",
    "username": "admin",
    "roles": ["admin"],
    "permissions": ["stock.read", "stock.write", "user.manage", "user.register"]
  }
}
```

失败返回：

```text
401 invalid_credentials
```

### `POST /api/auth/refresh`

输入：

```json
{
  "refresh_token": "<opaque-token>"
}
```

成功返回新的 access token 和 refresh token。

行为要求：

- 旧 refresh token 标记为 revoked/replaced。
- 新 refresh token 只在响应中返回一次。
- 旧 refresh token 后续再次使用必须失败。

失败返回：

```text
401 invalid_refresh_token
```

### `POST /api/auth/logout`

输入：

```json
{
  "refresh_token": "<opaque-token>"
}
```

成功行为：

- 吊销当前 refresh token。
- 返回 `204 No Content`。

### `GET /api/auth/me`

认证方式：

```http
Authorization: Bearer <access_token>
```

成功返回当前用户、角色和权限。

## Token 结构

JWT claims：

```text
sub: user_id
jti: access_token_id
iat: issued_at
exp: expires_at
kid: signing key id in JWT header
roles: role list
permissions: permission list
```

JWT 中的角色和权限是签发时快照。需要当前授权状态的管理接口必须在校验 token 后重新读取数据库权限；`GET /api/auth/me` 也返回数据库当前角色和权限。

JWT signing key 由系统初始化数据库时生成并保存到 `auth_signing_keys`，不放入 JSON 配置文件。
内置 RBAC 定义先于 JWT signing key 初始化补齐；首个注册用户只自动获得 `admin` 角色，具体权限来自 `admin` 角色的内置角色权限关系。

refresh token 服务端存储记录：

```text
id
user_id
token_hash
device_name
client_kind
created_at
expires_at
last_used_at
revoked_at
```

## 实现边界

- 在 `core` 中新增 auth 模块，封装登录、刷新、登出、当前用户提取和鉴权错误。
- 使用 `jsonwebtoken` 签发和校验 HS256 JWT；密钥来自数据库中的 `auth_signing_keys`，不能硬编码，不能由用户在 JSON 中配置。
- 使用 `argon2` 校验密码哈希。
- 使用加密安全随机数生成 refresh token；客户端只看到明文一次。
- 实现 `CurrentUser` Axum extractor，从 `Authorization` header 读取 Bearer token，校验后提供 `user_id`、`roles`、`permissions`。
- 受保护路由通过 extractor 或权限中间件获取当前用户，不直接解析 token。
- 业务授权判断权限代码，不判断角色代码；需要当前授权状态的管理接口应重新读取数据库权限。
- OpenAPI 文档标注 Bearer auth security scheme，并把 auth routes 纳入 Swagger UI。
- 不选择具体数据库引擎；方案要求 `core` 依赖一个 refresh token/user repository 抽象，后续由持久化实现落地。

## 安全规则

- access token 不入库，过期后只能通过 refresh token 换新。
- refresh token 入库前必须哈希，不能明文保存。
- refresh token 每次刷新都轮换。
- 检测到已吊销 token 被复用时返回 401。
- 密码只保存 `argon2` 哈希。
- 所有认证失败统一返回 401，不暴露用户名是否存在。
- 权限不足返回 403。
- JWT signing key 必须由系统生成并保存在数据库内部表，不允许写死在源码或暴露给用户配置。
- 生产或 LAN/server-mode 场景应使用 HTTPS；本地 self-hosted 可使用 `127.0.0.1` HTTP。

## 测试场景

- 登录成功返回 access token 和 refresh token。
- 首个注册用户无需 token 且自动获得 admin 角色。
- 首个注册用户经由内置 admin 角色获得 `stock.read`、`stock.write`、`user.manage` 和 `user.register` 权限。
- 已有用户后注册新用户必须由当前拥有 `user.register` 权限的用户调用。
- 密码错误返回 401。
- access token 可访问受保护接口。
- 缺少 Bearer token 返回 401。
- 过期或签名错误 access token 返回 401。
- refresh 成功后旧 refresh token 失效。
- 重复使用旧 refresh token 返回 401。
- logout 后 refresh token 不可再用。
- 无权限用户访问受限接口返回 403。
- OpenAPI 中能看到 Bearer auth 和 auth endpoints。

## 当前假设

- v1 登录方式为用户名 + 密码。
- v1 支持正式多用户、角色和权限。
- access token 有效期固定为 15 分钟。
- refresh token 有效期固定为 7 天。
- token 传输采用 JSON response + `Authorization: Bearer` header。
- JWT signing key 不写入 JSON 配置，由数据库 bootstrap 自动生成和管理。
- 内置 RBAC 定义由数据库 bootstrap 自动补齐，但 bootstrap 不创建用户。
- 不在本方案中绑定 SQLite、PostgreSQL 或其他具体持久化引擎。
- 不使用 cookie session，不引入 `axum-login`。
