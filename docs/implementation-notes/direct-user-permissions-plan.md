# 直接用户权限模型实施方案

本文档记录将当前“用户 -> 角色 -> 权限”模型收敛为“用户 -> 权限”模型的实施方案。
该方案面向当前 server/API 优先阶段，目标是降低用户管理认知成本，并让权限配置与实际业务能力一一对应。

## 背景

当前模型通过角色间接授予权限：

```text
auth_users
  -> auth_user_role_assignments
  -> auth_roles
  -> auth_role_permission_assignments
  -> auth_permissions
```

这套模型在存在组织、部门、岗位、用户组或大量重复授权模板时有价值。
但当前 WineStock 只有少量内置角色和一组明确业务权限，`admin`、`staff`、`viewer` 容易被误解为业务等级。
在权限已经按 `stock.*`、`user.*` 等能力拆细后，继续让用户先选择角色再间接获得权限，会增加不必要的解释成本。

## 目标

- 用户直接拥有权限，不再通过角色继承权限。
- 用户详情、登录响应和当前用户响应只返回权限列表，不返回角色列表。
- 用户管理界面可以直接勾选权限。
- 首个用户直接获得全部内置权限。
- 保留 `auth_permissions` 作为系统权限定义表。
- 删除或废弃角色相关 API 和持久化关系。
- 防止移除最后一个可管理权限的 active 用户，避免系统锁死。

## 非目标

- 不引入组织、部门、岗位或用户组。
- 不引入权限模板作为核心授权模型。
- 不在 JSON 启动配置中定义权限。
- 不让业务代码判断角色名或角色等级。
- 不把权限直接写入 JWT 后长期信任；管理接口仍应在 route layer 重新读取数据库当前权限。

## 目标数据模型

保留：

```text
auth_users
auth_permissions
auth_refresh_tokens
auth_settings
auth_signing_keys
```

新增：

```text
auth_user_permission_assignments
```

建议字段：

```text
user_id INTEGER NOT NULL
permission_id INTEGER NOT NULL
created_at TEXT NOT NULL
PRIMARY KEY (user_id, permission_id)
FOREIGN KEY (user_id) REFERENCES auth_users(id) ON DELETE CASCADE
FOREIGN KEY (permission_id) REFERENCES auth_permissions(id) ON DELETE CASCADE
```

移除或废弃：

```text
auth_roles
auth_user_role_assignments
auth_role_permission_assignments
```

如果需要兼容已有开发数据库，可以先新增 `auth_user_permission_assignments` 并迁移数据，再在后续迁移中删除角色表。
如果确认没有需要保留的数据库，可以直接改初始 schema。

## 权限代码

继续保留库存和审计权限：

```text
stock.read
stock.write
stock.item.manage
stock.item.read
stock.template.manage
stock.template.read
stock.inbound.create
stock.inbound.read
stock.inbound.approve
stock.outbound.create
stock.outbound.read
stock.outbound.approve
stock.substitute.manage
stock.substitute.read
stock.dashboard.read
audit.read
```

用户域建议权限：

| 权限代码 | 含义 |
| --- | --- |
| `user.register` | 注册新用户 |
| `user.read` | 查看用户列表和用户详情 |
| `user.status.update` | 启用或停用用户账号 |
| `user.permissions.update` | 整体替换用户权限 |
| `user.permission.read` | 查看权限定义 |
| `user.password.reset` | 设置其他用户临时密码 |

说明：

- `user.role.read` 和 `user.roles.update` 应随角色模型删除而移除。
- `user.permissions.update` 替代“给用户分配角色”的能力。
- 当前用户修改自己密码仍只要求已登录并校验当前密码，不需要额外权限。

## HTTP API 调整

保留：

```text
POST /api/auth/register
POST /api/auth/login
POST /api/auth/refresh
POST /api/auth/logout
GET  /api/auth/me
POST /api/auth/me/password
GET  /api/users
GET  /api/users/{id}
PATCH /api/users/{id}/status
POST /api/users/{id}/password
GET  /api/permissions
```

新增或替换：

```text
PUT /api/users/{id}/permissions
```

删除：

```text
GET /api/roles
PUT /api/users/{id}/roles
```

目标授权：

| 接口 | 权限 |
| --- | --- |
| `GET /api/users` | `user.read` |
| `GET /api/users/{id}` | `user.read` |
| `PATCH /api/users/{id}/status` | `user.status.update` |
| `PUT /api/users/{id}/permissions` | `user.permissions.update` |
| `POST /api/users/{id}/password` | `user.password.reset` |
| `GET /api/permissions` | `user.permission.read` |

## 响应 DTO 调整

`AuthUserResponse` 删除 `roles` 字段：

```json
{
  "id": "1",
  "username": "admin",
  "permissions": ["stock.item.read", "user.read"]
}
```

`UserAdminResponse` 删除 `roles` 字段：

```json
{
  "id": 1,
  "username": "admin",
  "status": "active",
  "permissions": ["stock.item.read", "user.read"],
  "created_at": "...",
  "updated_at": "..."
}
```

JWT claims 删除 `roles` 字段，只保留 `permissions`。
需要当前授权状态的管理接口继续在 middleware 中重新读取数据库权限，不能只依赖 access token 中的权限快照。

## 首个用户初始化

首个用户注册时：

1. 在事务内判断数据库是否已有用户。
2. 创建用户。
3. 读取全部内置权限 ID。
4. 给首个用户写入全部权限。
5. 提交事务。

后续注册用户默认不授予任何权限，必须由拥有 `user.permissions.update` 的用户显式分配。

## 防锁死规则

替代当前“最后 active admin”保护：

- 禁止停用最后一个拥有 `user.permissions.update` 的 active 用户。
- 禁止从最后一个拥有 `user.permissions.update` 的 active 用户身上移除该权限。
- 可选增强：同时要求系统至少存在一个 active 用户拥有 `user.read`、`user.status.update`、`user.password.reset` 和 `user.register`。

推荐先采用最小规则：保护 `user.permissions.update`。
因为只要系统里还有一个 active 用户能分配权限，就可以修复其他授权问题。

## Repository 调整

`RbacRepository` 可以收敛为权限定义和用户权限分配仓储。

需要保留或新增的能力：

- `ensure_permission(code, description)`
- `list_permissions()`
- `list_user_permissions(user_id)`
- `replace_user_permissions(user_id, permission_ids)`
- `find_permission_ids_by_codes(codes)`
- `assign_permission_to_user(user_id, permission_id)`
- `has_other_active_user_with_permission(excluded_user_id, permission_code)`

需要移除的能力：

- `ensure_role`
- `list_roles`
- `list_user_roles`
- `list_role_permissions`
- `assign_role_to_user`
- `replace_user_roles`
- `sync_role_permissions`
- `has_other_active_admin`

## 迁移策略

### 方案 A：未发布或可丢弃数据库

直接修改初始 schema：

- 删除角色相关表。
- 新增 `auth_user_permission_assignments`。
- 修改测试和文档。

优点：实现简单，代码干净。
缺点：已有开发数据库需要删除重建。

### 方案 B：需要保留已有数据库

新增迁移：

1. 创建 `auth_user_permission_assignments`。
2. 将旧角色权限展开为用户直接权限：

```text
auth_user_role_assignments
  -> auth_role_permission_assignments
  -> auth_user_permission_assignments
```

3. 首个迁移版本保留旧角色表但业务代码不再读取。
4. 后续确认无回滚需求后删除旧角色表。

优点：已有数据库平滑升级。
缺点：会短期保留废弃表。

当前项目如果还没有需要保留的正式数据，推荐方案 A。

## 实施步骤

1. 修改 shared DTO：
   - `AuthUserResponse` 删除 `roles`。

2. 修改 JWT claims：
   - 删除 `roles`。
   - 登录和 refresh 只写入权限快照。

3. 修改 migration：
   - 移除角色表或新增用户权限关系表。

4. 修改 repository：
   - 查询用户权限改为直接查询 `auth_user_permission_assignments`。
   - 增加整体替换用户权限的仓储函数。

5. 修改 RBAC bootstrap：
   - 只补齐 `auth_permissions`。
   - 不补齐角色和角色权限关系。

6. 修改注册服务：
   - 首个用户直接授予全部内置权限。

7. 修改 users 路由：
   - 删除 `/api/roles`。
   - 删除 `/api/users/{id}/roles`。
   - 新增 `/api/users/{id}/permissions`。

8. 修改 users service：
   - 用户响应组装不再读取角色。
   - 增加整体替换用户权限逻辑。
   - 防锁死逻辑改为保护最后一个拥有 `user.permissions.update` 的 active 用户。

9. 修改 OpenAPI：
   - 删除角色相关 handler/schema。
   - 新增用户权限替换请求 DTO。

10. 修改测试：
    - 首个用户拥有全部内置权限。
    - 普通用户无权限时无法访问受保护接口。
    - 拥有具体权限的用户只能访问对应接口。
    - 替换用户权限会立即影响旧 token 后续请求。
    - 不能移除最后一个权限管理员。
    - 密码、自助改密和审计脱敏规则不变。

11. 修改文档：
    - `docs/rbac-permission-model.md`
    - `docs/user-management-api.md`
    - `docs/database-schema.md`
    - `docs/code-map.md`
    - `docs/validation/*`

## 验证清单

至少运行：

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace
cargo build -p winestock-server
```

重点测试：

- 首个用户注册后可完成全部管理操作。
- 非授权用户访问用户管理接口返回 403。
- 修改用户权限后，旧 access token 不能继续绕过已撤销权限。
- 禁止移除最后一个 active 用户的 `user.permissions.update`。
- `GET /api/auth/me` 和登录响应不再出现 `roles`。
- OpenAPI 不再包含 `/api/roles` 和 `/api/users/{id}/roles`。

## 风险和取舍

- 直接权限会让单个用户配置更细，但用户多时重复勾选会变多。
- 如果未来需要批量模板，可以增加“权限模板”或“用户组”，但不要让业务授权重新依赖角色等级。
- 删除 `roles` 是 API 破坏性变更，前端和客户端需要同步更新。
- 如果保留旧数据库，需要明确迁移策略，避免用户失去已有管理能力。
