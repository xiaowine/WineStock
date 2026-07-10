# 用户管理 API

本文档记录当前已实现的用户管理、当前用户修改自己密码和权限只读 HTTP API。

## 授权规则

- 业务授权判断权限代码，不判断角色代码。
- 用户直接拥有权限，响应体不返回角色列表。
- 管理接口会在 route layer 重新读取数据库当前权限，撤销权限后旧 access token 不能继续绕过授权。
- 当前用户修改自己密码只要求已登录并校验当前密码。
- 管理员设置临时密码后，目标用户登录响应会返回 `password_change_required = true`；该用户只能访问 `/api/auth/me` 和 `/api/auth/me/password`，改密成功后恢复正常访问。

## 用户域权限

| 权限代码 | 含义 |
| --- | --- |
| `user.register` | 注册新用户 |
| `user.read` | 查看用户列表和用户详情 |
| `user.status.update` | 启用或停用用户账号 |
| `user.permissions.update` | 整体替换用户权限 |
| `user.permission.read` | 查看权限定义 |
| `user.password.reset` | 设置其他用户临时密码 |

## DTO

### `UserAdminResponse`

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | integer | 用户 ID |
| `username` | string | 登录用户名 |
| `status` | `active`/`disabled` | 用户状态 |
| `permissions` | string[] | 用户直接拥有的权限代码 |
| `password_change_required` | boolean | 是否必须先修改临时密码 |
| `created_at` | string | 创建时间 |
| `updated_at` | string | 更新时间 |

### `PermissionResponse`

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `code` | string | 稳定权限代码 |
| `description` | string/null | 权限说明 |

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
  - `404 user_not_found`
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
- 响应：`200` + `UserAdminResponse`
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token`
  - `403 permission_denied`
  - `403 self_protected_permissions_update_forbidden`：当前操作者不能增加、移除自己的 `user.permissions.update` 或 `user.permission.read`。
  - `404 user_not_found` 或 `permission_not_found`
  - `409 last_permission_manager_required`

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
  - `404 user_not_found`

### `POST /api/auth/me/password`

当前用户修改自己的密码。

- 权限：已登录。
- 请求：

```json
{
  "current_password": "old-password",
  "new_password": "new-password"
}
```

- 响应：`204`
- 失败：
  - `400 invalid_request`
  - `401 invalid_access_token` 或 `invalid_credentials`

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

## 审计

- 账号启停：`entity_type = "user"`，`action = "updated"`，详情记录旧状态和新状态。
- 用户权限替换：`entity_type = "user"`，`action = "updated"`，详情记录旧权限和新权限。
- 当前用户修改自己密码：`entity_type = "user"`，`action = "updated"`，详情只记录字段名和 `self_change` 模式。
- 管理员设置临时密码：`entity_type = "user"`，`action = "updated"`，详情只记录字段名、`admin_temporary_password` 模式和强制改密标记。
- 审计详情不得包含明文密码、token 或密码哈希。
