# WineStock 用户管理 API 文档

本文档记录当前已实现的用户管理和 RBAC 只读接口。
用户注册、登录、刷新、登出和当前用户接口仍见鉴权相关实现文档；本文只描述管理员管理用户所需的 HTTP API。

## 设计边界

- 用户管理属于 `core` 的 `users` 业务模块。
- 路由鉴权由 `AuthorizeRouteExt` 在进入 handler 前完成。
- 管理接口统一要求 `user.manage` 权限。
- 业务授权判断权限代码，不判断角色代码；`admin` 只是内置角色模板。
- 角色和权限首版只读，不提供创建、修改或删除角色/权限接口。
- 用户没有直接绑定权限的表；用户权限来自用户拥有的角色。
- 管理写操作会写入 `audit_events`，但不得记录明文密码、密码哈希、JWT、refresh token 或签名密钥。

## 权限

| 权限代码 | 用途 |
| --- | --- |
| `user.register` | 注册新用户；首个用户免鉴权，已有用户后需要该权限 |
| `user.manage` | 查询用户、启停账号、分配角色、重置密码、查看角色和权限定义 |

## 数据结构

### `UserAdminResponse`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | integer | 用户数据库 ID |
| `username` | string | 登录用户名 |
| `display_name` | string/null | 展示名称 |
| `status` | string | `active` 或 `disabled` |
| `roles` | string[] | 用户直接拥有的角色代码 |
| `permissions` | string[] | 用户经由角色获得的权限代码 |
| `created_at` | string | SQLite UTC 时间字符串 |
| `updated_at` | string | SQLite UTC 时间字符串 |

### `RoleResponse`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `code` | string | 稳定角色代码 |
| `name` | string | 角色名称 |
| `description` | string/null | 角色说明 |
| `permissions` | string[] | 该角色包含的权限代码 |

### `PermissionResponse`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `code` | string | 稳定权限代码 |
| `description` | string/null | 权限说明 |

## 接口

### `GET /api/users`

分页查询用户列表。

- 权限：`user.manage`
- 查询参数：
  - `page`：页码，默认 `1`
  - `page_size`：每页数量，默认 `50`，最大 `200`
  - `search`：按用户名或展示名模糊搜索
  - `status`：按 `active` 或 `disabled` 筛选
  - `role`：按角色代码筛选
- 响应：`200` + `PaginatedResponse<UserAdminResponse>`

### `GET /api/users/{id}`

查询单个用户管理详情。

- 权限：`user.manage`
- 响应：`200` + `UserAdminResponse`
- 错误：`404 user_not_found`

### `PATCH /api/users/{id}/status`

启用或停用用户。

- 权限：`user.manage`
- 请求：

```json
{
  "status": "disabled"
}
```

- 响应：`200` + `UserAdminResponse`
- 错误：
  - `400 invalid_request`
  - `404 user_not_found`
  - `409 last_admin_required`

停用用户后，该用户现有 access token 和 refresh token 会因为数据库中的用户状态不是 `active` 而被拒绝。

### `PUT /api/users/{id}/roles`

整体替换用户角色。

- 权限：`user.manage`
- 请求：

```json
{
  "roles": ["staff"]
}
```

- 空数组表示清空该用户角色。
- 响应：`200` + `UserAdminResponse`
- 错误：
  - `400 invalid_request`
  - `404 user_not_found` 或 `role_not_found`
  - `409 last_admin_required`

### `POST /api/users/{id}/password`

管理员重置用户密码。

- 权限：`user.manage`
- 请求：

```json
{
  "password": "new-password"
}
```

- 密码长度：8 到 128，trim 后不能为空。
- 响应：`204 No Content`
- 错误：
  - `400 invalid_request`
  - `404 user_not_found`

响应和审计事件都不会返回或记录明文密码、密码哈希或 token。

### `GET /api/roles`

查询角色定义列表。

- 权限：`user.manage`
- 响应：`200` + `RoleResponse[]`

### `GET /api/permissions`

查询权限定义列表。

- 权限：`user.manage`
- 响应：`200` + `PermissionResponse[]`

## 安全规则

- 禁止停用最后一个 `active` 的 `admin` 用户。
- 禁止从最后一个 `active` 的 `admin` 用户身上移除 `admin` 角色。
- 这里的 `admin` 不具备硬编码超级权限，只是包含当前全部内置权限的角色模板。
- 管理接口会在 route layer 重新读取数据库中的当前权限，旧 JWT 中的权限快照不能绕过已撤销的授权。

## 审计

以下操作写入 `audit_events`：

- 用户状态更新：`entity_type = "user"`，`action = "updated"`，详情记录旧状态和新状态。
- 用户角色替换：`entity_type = "user"`，`action = "updated"`，详情记录旧角色和新角色。
- 管理员重置密码：`entity_type = "user"`，`action = "updated"`，详情只记录字段名为 `password`。

审计详情必须保持脱敏，不能包含密码明文、密码哈希、JWT、refresh token 或签名密钥。
