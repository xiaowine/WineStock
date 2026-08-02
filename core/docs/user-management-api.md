# 用户管理 API

本文档记录当前已实现的用户管理、当前用户修改自己密码和权限只读 HTTP API。

## 授权规则

- 业务授权判断权限代码，不判断角色代码。
- 用户直接拥有权限，响应体不返回角色列表。
- 管理接口会在 route layer 重新读取数据库当前权限，撤销权限后旧 access token 不能继续绕过授权。
- 当前用户修改自己密码只要求已登录并校验当前密码；请求中的 `username` 为必填，普通改密时传入当前用户名。
- 管理员设置临时密码后，目标用户登录响应会返回 `password_change_required = true`；该用户只能访问 `/api/auth/me` 和 `/api/auth/me/password`，改密成功后恢复正常访问。

## 用户域权限

| 权限代码                  | 含义                   |
| ------------------------- | ---------------------- |
| `user.register`           | 注册新用户             |
| `user.read`               | 查看用户列表和用户详情 |
| `user.status.update`      | 启用或停用用户账号     |
| `user.delete`             | 软删除其他用户账号     |
| `user.permissions.update` | 整体替换用户权限       |
| `user.permission.read`    | 查看权限定义           |
| `user.password.reset`     | 设置其他用户临时密码   |
| `user.username.update`    | 修改用户登录用户名     |

## DTO

### `UserAdminResponse`

| 字段                       | 类型                | 含义                   |
| -------------------------- | ------------------- | ---------------------- |
| `id`                       | integer             | 用户 ID                |
| `username`                 | string              | 登录用户名             |
| `status`                   | `active`/`disabled` | 用户状态               |
| `permissions`              | string[]            | 用户直接拥有的权限代码 |
| `password_change_required` | boolean             | 是否必须先修改临时密码 |
| `created_at`               | string              | 创建时间               |
| `updated_at`               | string              | 更新时间               |

### `PermissionResponse`

| 字段          | 类型        | 含义         |
| ------------- | ----------- | ------------ |
| `code`        | string      | 稳定权限代码 |
| `description` | string/null | 权限说明     |

## 接口

### `GET /api/users`

分页查询用户列表。

- 权限：`user.read`
- 查询参数：
  - `page`：页码，默认 1。
  - `page_size`：每页数量，默认 50，最大 200。
  - `search`：按用户名模糊搜索。
  - `status`：按 `active` 或 `disabled` 筛选。
- 响应：`200` + `PaginatedResponse<UserAdminResponse>`
- 失败：
  - `401 invalid_access_token`
  - `403 permission_denied`

### `GET /api/users/{id}`

查询单个用户详情。

- 权限：`user.read`
- 响应：`200` + `UserAdminResponse`
- 失败：
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `404 user_not_found`

### `PATCH /api/users/{id}/status`

启用或停用用户账号。

- 权限：`user.status.update`
- 请求：

```json
{
  "status": "disabled"
}
```

- 响应：`200` + `UserAdminResponse`
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `403 self_status_update_forbidden`：禁止停用当前操作者自己的账号。
  - `404 user_not_found`
  - `409 last_permission_manager_required`

### `DELETE /api/users/{id}`

软删除其他用户账号。

- 权限：`user.delete`
- 响应：`204`
- 行为：
  - 将目标账号状态置为 `disabled` 并写入 `deleted_at`。
  - 吊销目标用户全部 active refresh token；其现有 access token 也会因用户查询不可见而立即失效。
  - 用户列表、详情、登录、refresh 和授权查询不再返回该用户。
  - 保留用户行、权限分配和历史业务关联；用户名继续占用，不能重新注册复用。
- 失败：
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `403 self_user_delete_forbidden`：禁止删除当前操作者自己的账号。
  - `404 user_not_found`：用户不存在或已经软删除。
  - `409 last_permission_manager_required`

### `PUT /api/users/{id}/permissions`

整体替换用户直接拥有的权限。

- 权限：`user.permissions.update`
- 请求：

```json
{
  "permissions": ["stock.item.read", "user.read"]
}
```

- 空数组表示清空该用户权限。
- 所有权限代码必须已存在于 `auth_permissions`；任一代码不存在时整体返回 `404 permission_not_found`，不会写入部分有效权限。
- 响应：`200` + `UserAdminResponse`
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `403 self_protected_permissions_update_forbidden`：当前操作者不能增加、移除自己的 `user.permissions.update` 或 `user.permission.read`。
  - `404 user_not_found` 或 `permission_not_found`
  - `409 last_permission_manager_required`

### `PATCH /api/users/{id}/username`

修改用户登录用户名。

- 权限：`user.username.update`
- 请求：

```json
{
  "username": "new-name"
}
```

- 响应：`200` + `UserAdminResponse`
- 行为：用户 ID、权限、业务关联、JWT、refresh token 和现有会话保持不变；旧用户名立即不能登录，新用户名立即生效。
- 失败：`400 invalid_request`、`401 invalid_access_token`、`403 permission_denied`、`404 user_not_found`、`409 username_taken`

### `POST /api/users/{id}/password`

管理员为其他用户设置临时密码。

- 权限：`user.password.reset`
- 请求：

```json
{
  "password": "temporary-password"
}
```

- 响应：`204`
- 行为：
  - 目标用户现有 refresh token 会被吊销。
  - 目标用户使用临时密码登录后，响应中的 `password_change_required` 为 `true`。
  - 目标用户在改密前只能访问 `/api/auth/me` 和 `/api/auth/me/password`，其他已鉴权接口返回 `403 password_change_required`。
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `403 self_password_reset_forbidden`：禁止为当前操作者自己设置临时密码。
  - `404 user_not_found`

### `POST /api/auth/me/password`

当前用户修改自己的密码。

- 权限：已登录。
- 请求：

```json
{
  "username": "current-name",
  "current_password": "old-password",
  "new_password": "new-password"
}
```

- 响应：`204`
- 行为：
  - `username` 是必填字段；普通改密时传入当前用户名，self-hosted 占位密码首次设置时可同时修改用户名。
  - 用户名、密码哈希和强制改密状态在同一事务中更新；用户名冲突或密码校验失败时两者均不写入。
  - 唯一例外：本机免登录标记用户的密码仍为自动开通的随机占位值时（`local_auto_login_password_placeholder = true`），
    允许 `current_password` 留空直接设置新密码；成功后清除占位标记。
  - 针对标记用户的任何改密（本人改密或管理员重置临时密码）都会清除占位标记。
  - 见 `docs/implementation-notes/self-hosted-silent-auth.md`。
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token` 或 `invalid_credentials`

### `POST /api/auth/local-session`（匿名）与 `GET /api/auth/local-session/status`（已登录）

self-hosted 本机静默会话换取与占位密码状态查询。

- `local-session`：请求携带壳内可信通道下发的 per-boot 换取凭据（`exchange_token` + 设备元数据），
  成功返回与登录相同的 token 包。空库首次换取会自动开通 `admin`（随机占位密码 + 全部内置权限，
  与首用户注册共用写锁）；标记用户被停用/软删除/收权时自愈并写审计。
- 失败：`401 invalid_credentials`（凭据不匹配）；`404 local_session_unavailable`（非 self-hosted 模式，
  或已有用户但鉴权设置未标记换取目标——存量库需手工插入
  `local_auto_login_user_id` 设置行转换）。
- `local-session/status`：返回 `{ "password_placeholder": bool }`，仅当前用户就是标记用户且
  密码仍为占位时为 true；前端切 `server-mode` 前据此强制设密。

### `GET /api/permissions`

查询权限定义列表。

- 权限：`user.permission.read`
- 响应：`200` + `PermissionResponse[]`
- 失败：
  - `401 invalid_access_token`
  - `403 permission_denied`

## 防锁死规则

- 禁止停用最后一个拥有 `user.permissions.update` 的 active 用户。
- 禁止从最后一个拥有 `user.permissions.update` 的 active 用户身上移除该权限。
- 只要还有一个 active 用户能分配权限，就可以修复其他授权问题。
- 当前操作者更新自己的权限时，不能增加或移除自己的 `user.permissions.update` 和 `user.permission.read`；其他自身权限仍可调整。

## 当前操作者保护

- 后端拒绝当前操作者停用自己的账号，前端隐藏入口不是安全边界。
- 后端拒绝当前操作者软删除自己的账号，并保护最后一个 active 权限管理员。
- 后端拒绝当前操作者为自己设置临时密码；当前用户修改自己的密码必须使用 `/api/auth/me/password`。
- 这些规则在用户管理 service 层执行，发生在状态写入、软删除、密码哈希、token 吊销和审计写入之前。

## 审计

- 账号启停：`entity_type = "user"`，`action = "updated"`，详情记录旧状态和新状态。
- 用户软删除：`entity_type = "user"`，`action = "deleted"`，详情记录软删除模式和删除前状态。
- 用户权限替换：`entity_type = "user"`，`action = "updated"`，详情记录旧权限和新权限。
- 当前用户修改自己密码：`entity_type = "user"`，`action = "updated"`，详情只记录字段名和 `self_change` 模式。
- 用户名修改：`entity_type = "user"`，`action = "updated"`，详情记录 `previous_username` 和 `new_username`。
- 管理员设置临时密码：`entity_type = "user"`，`action = "updated"`，详情只记录字段名、`admin_temporary_password` 模式和强制改密标记。
- 审计详情不得包含明文密码、token 或密码哈希。
