# `core/src/users/controller.rs`

本文件定义用户业务 HTTP DTO 和 handler。

## 用户管理查询和响应

- `UserListQuery.page`：可空；服务层归一为最小 1。
- `UserListQuery.page_size`：可空；服务层默认 50，最大 200。
- `UserListQuery.search`：可空；存在时 trim 后不能为空，用于用户名或展示名模糊搜索。
- `UserListQuery.status`：可空；存在时只允许 `active` 或 `disabled`。
- `UserAdminResponse`：返回用户 ID、用户名、展示名、状态、权限、强制改密标记、创建时间和更新时间；不返回密码哈希。

## 写请求 DTO

- `UserStatusUpdateRequest.status`：只允许 `active` 或 `disabled`。
- `UserPermissionsUpdateRequest.permissions`：最多 32 个权限代码；每项必须满足共享代码格式；空列表表示清空权限。
- `UserPasswordResetRequest.password`：长度 8 到 128，trim 后非空；作为管理员设置的临时密码，只允许出现在请求体中，服务端只保存 Argon2 哈希。
- `UserPasswordChangeRequest.current_password`：长度 1 到 256，trim 后非空；只用于校验当前登录用户仍掌握原凭据。
- `UserPasswordChangeRequest.new_password`：长度 8 到 128，trim 后非空；只允许出现在请求体中，服务端只保存 Argon2 哈希。

## 管理接口约束

- `/api/users` 和 `/api/users/{id}` 由路由层要求 `user.read`。
- `/api/users/{id}/status` 由路由层要求 `user.status.update`。
- `/api/users/{id}/permissions` 由路由层要求 `user.permissions.update`。
- `/api/permissions` 由路由层要求 `user.permission.read`。
- `/api/users/{id}/password` 由路由层要求 `user.password.reset`，用于拥有重置权限的用户设置目标用户临时密码，并要求目标用户下次登录后改密。
- `/api/auth/me/password` 只要求已登录，服务层校验当前密码，并且不接受其他用户 ID。
- 账号启停、权限替换、当前用户修改自己密码和管理员设置临时密码会写入 `audit_events`；审计详情不得包含明文密码、token 或密码哈希。
- 禁止禁用最后一个拥有 `user.permissions.update` 的 active 用户，也禁止移除最后一个 active 权限管理员的 `user.permissions.update` 权限。
