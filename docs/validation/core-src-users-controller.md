# `core/src/users/controller.rs`

本文件定义用户业务 HTTP DTO 和 handler。

## 用户管理查询和响应

- `UserListQuery.page`：可空；服务层归一为最小 1。
- `UserListQuery.page_size`：可空；服务层默认 50，最大 200。
- `UserListQuery.search`：可空；存在时 trim 后不能为空，用于用户名或展示名模糊搜索。
- `UserListQuery.status`：可空；存在时只允许 `active` 或 `disabled`。
- `UserListQuery.role`：可空；存在时按角色代码筛选。
- `UserAdminResponse`：返回用户 ID、用户名、展示名、状态、角色、权限、创建时间和更新时间；不返回密码哈希。

## 写请求 DTO

- `UserStatusUpdateRequest.status`：只允许 `active` 或 `disabled`。
- `UserRolesUpdateRequest.roles`：最多 32 个角色代码；每项必须满足共享代码格式；空列表表示清空角色。
- `UserPasswordResetRequest.password`：长度 8 到 128，trim 后非空；只允许出现在请求体中，服务端只保存 Argon2 哈希。

## 管理接口约束

- `/api/users*`、`/api/roles` 和 `/api/permissions` 均由路由层要求 `user.manage`。
- 账号启停、角色替换和重置密码会写入 `audit_events`；审计详情不得包含明文密码、token 或密码哈希。
- 禁止禁用最后一个 active admin，也禁止移除最后一个 active admin 的 `admin` 角色。
