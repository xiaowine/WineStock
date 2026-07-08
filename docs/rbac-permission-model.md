# 用户直接权限模型

本文档定义 WineStock 当前授权模型、启动初始化行为和业务授权规则。

当前模型是“用户 -> 权限”。业务授权统一判断权限代码，不判断角色代码。
当前实现不包含 `admin`、`staff`、`viewer` 等角色模板，也不提供角色相关 API。

## 代码所有权

- `core/src/stock/permissions.rs` 定义库存和审计业务权限代码。
- `core/src/users/permissions.rs` 定义用户管理权限代码。
- `core/src/rbac/bootstrap.rs` 定义内置权限清单，并在本地服务启动时补齐缺失权限定义。
- `core/src/persistence/repository/rbac_repo.rs` 封装权限定义查询和用户权限分配表。
- `core/src/security/middleware.rs` 在进入业务 handler 前重新读取数据库当前权限。

`shared` 只定义 API 契约中的 `permissions` 字段，不拥有授权规则。

## 数据模型

授权使用以下业务表：

```text
auth_users
  -> auth_user_permission_assignments
  -> auth_permissions
```

- `auth_users`：账号基础资料。
- `auth_permissions`：系统权限定义，保存稳定权限代码和说明。
- `auth_user_permission_assignments`：用户直接拥有的权限。

`auth_permissions` 是权限字典，不是授权关系。一个用户是否拥有权限只看 `auth_user_permission_assignments`。

## 启动初始化

本地服务启动流程：

```text
打开 SQLite
执行 migration
补齐内置权限定义
初始化鉴权设置和 JWT signing key
```

权限初始化是幂等的：

- 只补齐缺失权限。
- 不覆盖已有权限说明。
- 不创建用户。
- 不写入任何用户权限分配。

## 内置权限

库存和审计权限：

| 权限代码 | 含义 |
| --- | --- |
| `stock.read` | 查看库存数据 |
| `stock.write` | 创建或修改库存数据 |
| `stock.item.manage` | 创建、修改和软删除库存物品 |
| `stock.template.manage` | 管理库存模板和模板字段 |
| `stock.inbound.create` | 创建入库单 |
| `stock.inbound.approve` | 审批或拒绝入库单 |
| `stock.outbound.create` | 创建出库单 |
| `stock.outbound.approve` | 审批或拒绝出库单 |
| `stock.substitute.manage` | 绑定或解绑替代料关系 |
| `audit.read` | 查询审计事件日志 |

用户管理权限：

| 权限代码 | 含义 |
| --- | --- |
| `user.register` | 注册新用户 |
| `user.read` | 查看用户列表和用户详情 |
| `user.status.update` | 启用或停用用户账号 |
| `user.permissions.update` | 整体替换用户权限 |
| `user.permission.read` | 查看权限定义 |
| `user.password.reset` | 设置其他用户临时密码 |

## 首个用户

首个注册用户在同一事务内完成：

1. 重新判断数据库是否已有用户。
2. 创建用户。
3. 读取全部内置权限 ID。
4. 给该用户写入全部内置权限。
5. 提交事务。

后续注册用户默认没有任何权限，必须由拥有 `user.permissions.update` 的 active 用户显式分配。

## Token 和授权

登录和 refresh 会把签发时的权限快照写入 JWT access token，并在响应体中返回当前用户权限列表。
JWT 不包含角色列表。

需要当前授权状态的管理接口必须在 route layer 重新读取数据库当前权限，不能只信任 access token 中的权限快照。
`GET /api/auth/me` 也返回数据库当前权限。

## 用户管理接口

| 接口 | 权限 |
| --- | --- |
| `GET /api/users` | `user.read` |
| `GET /api/users/{id}` | `user.read` |
| `PATCH /api/users/{id}/status` | `user.status.update` |
| `PUT /api/users/{id}/permissions` | `user.permissions.update` |
| `POST /api/users/{id}/password` | `user.password.reset` |
| `GET /api/permissions` | `user.permission.read` |

当前用户修改自己密码只要求已登录并校验当前密码。

为避免系统锁死，用户管理接口禁止禁用最后一个拥有 `user.permissions.update` 的 active 用户，也禁止从最后一个拥有该权限的 active 用户身上移除 `user.permissions.update`。

## 新增受保护能力

新增受保护能力时：

1. 在对应领域的 `permissions.rs` 中定义稳定权限代码，命名使用 `domain.action` 形式。
2. 在 `core/src/rbac/bootstrap.rs` 的内置权限中补齐权限说明。
3. 在对应业务模块的 `mod.rs` 路由装配处挂载权限 middleware，不在 handler 中判断角色或权限代码。
4. 更新本文档、OpenAPI 相关测试和用户管理文档。

## 禁止事项

- 不要用角色代码作为业务授权条件。
- 不要实现角色等级比较。
- 不要把 `admin` 写成隐含超级权限绕过。
- 不要在 handler 中直接拼接权限表关联 SQL。
- 不要在启动权限初始化中创建用户。
- 不要在 JSON 启动配置里定义权限或 JWT signing key。
