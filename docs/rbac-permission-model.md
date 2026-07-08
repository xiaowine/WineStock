# RBAC 权限模型

本文档定义 WineStock 当前 RBAC 模型、启动初始化行为和业务授权规则。

实现以 `core/src/rbac/bootstrap.rs`、`core/src/rbac/policy.rs`、`core/src/users/permissions.rs`、`core/src/security/middleware.rs`、`core/src/users/mod.rs`、`core/src/persistence/repository/rbac_repo.rs` 和数据库 migration 为准。

## 核心规则

业务授权统一判断权限代码，不判断角色代码。

角色只负责批量授予权限。角色不是业务授权等级，也不表达 `admin > staff > viewer` 这类层级关系。业务代码不得写“只要是 `admin` 角色就允许”的判断；需要受保护的能力必须定义明确权限，例如 `user.register` 或 `stock.write`。

## 职责边界

`core/src/rbac/policy.rs` 负责：

- 定义稳定的角色代码。

`core/src/stock/permissions.rs` 负责：

- 定义库存和审计相关稳定权限代码，例如 `stock.read`、`stock.item.manage` 和 `audit.read`。
- 让库存业务模块和 RBAC 启动逻辑共享同一份权限命名来源。

`core/src/users/permissions.rs` 负责：

- 定义用户域稳定权限代码，例如 `user.register`、`user.manage`。
- 让用户业务模块和 RBAC 启动逻辑共享同一份权限命名来源。

`core/src/rbac/bootstrap.rs` 负责：

- 定义内置角色。
- 定义内置权限。
- 定义内置角色包含哪些权限。
- 在本地服务启动时补齐缺失的 RBAC 基础数据。

`core/src/persistence/repository/rbac_repo.rs` 负责：

- 查询用户经由角色获得的权限代码。
- 查询用户直接分配到的角色代码。
- 补齐角色、权限和分配关系。
- 隔离 RBAC 表结构，避免 handler 直接拼接关联查询。

`core/src/security/middleware.rs` 负责：

- 在 Axum route layer 中完成 bearer token 校验。
- 在进入业务 handler 前重新读取数据库中的当前角色和权限。
- 根据路由声明执行“必须登录”“必须具备权限”或“条件权限”校验。

`core/src/users/mod.rs` 负责：

- 在用户业务路由注册处挂载权限 middleware。
- 把“数据库已有用户后注册接口需要 `user.register` 权限”这类条件鉴权规则表达为路由装配，而不是散落在 handler 中。
- 把用户列表、用户详情、账号启停、用户角色分配、管理员重置密码、角色只读和权限只读接口统一挂载 `user.manage` 权限。

`shared` 只定义 API 契约中的 `roles` 和 `permissions` 字段，不拥有 RBAC 规则。

## 数据库表

RBAC 使用以下业务表：

```text
auth_users
  -> auth_user_role_assignments
  -> auth_roles
  -> auth_role_permission_assignments
  -> auth_permissions
```

表职责：

- `auth_roles`：角色定义，保存角色代码、名称和说明。
- `auth_user_role_assignments`：用户拥有哪些角色。
- `auth_permissions`：权限定义，保存权限代码和说明。
- `auth_role_permission_assignments`：角色模板包含哪些权限。

当前没有“用户直接绑定权限”的表。用户权限来自用户拥有的角色，再经由角色权限关系计算。

## 启动初始化

本地服务启动顺序：

```text
打开存储
执行 migration
补齐内置 RBAC 定义
初始化鉴权设置和 JWT signing key
```

RBAC 初始化是幂等的：

- 只补齐缺失角色、权限和角色权限关系。
- 不创建任何用户。
- 不覆盖已有角色名称、角色说明或权限说明。
- 不删除数据库中额外存在的自定义角色、权限或分配关系。

这个顺序保证 JWT 签发前，数据库中已经存在可用于计算用户权限的基础角色和权限。

## 内置定义

内置角色：

| 角色代码 | 默认名称 | 含义 |
| --- | --- | --- |
| `admin` | `Admin` | 系统管理员角色模板 |
| `staff` | `Staff` | 日常业务操作角色模板 |
| `viewer` | `Viewer` | 只读访问角色模板 |

内置权限：

| 权限代码 | 含义 |
| --- | --- |
| `user.register` | 注册新用户 |
| `user.manage` | 管理用户、角色和权限 |
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

内置角色权限关系：

| 角色代码 | 权限 |
| --- | --- |
| `admin` | 当前全部内置权限 |
| `staff` | `stock.read`, `stock.write`, `stock.item.manage`, `stock.inbound.create`, `stock.outbound.create`, `stock.substitute.manage` |
| `viewer` | `stock.read`, `audit.read` |

## 首个用户

当数据库没有任何用户时，`POST /api/auth/register` 不要求 bearer token。

首个注册用户会自动分配 `admin` 角色。它不是因为 `admin` 角色本身拥有特殊授权等级，而是因为 `admin` 角色模板包含当前全部内置权限。

当数据库已经存在用户后，注册新用户必须由当前拥有 `user.register` 权限的 bearer token 调用。

这个规则不是注册接口专用的硬编码分支。路由注册处使用条件鉴权组合“数据库已有用户”条件和 `user.register` 权限；后续如果条件变化，只需要替换条件函数或权限代码。

注册服务会在同一事务内重新判断是否已有用户、创建账号并分配首个 `admin` 角色，避免并发首登请求都按“空库”路径获得管理员权限。

## JWT 与当前授权

登录和刷新时，access token 会携带签发时的 `roles` 和 `permissions` 快照。响应体也会返回当前用户的角色和权限列表，便于客户端展示。

需要当前授权状态的管理接口不能只依赖 JWT 快照。它应通过鉴权 middleware 在校验 bearer token 后重新读取数据库中的当前权限，再判断具体权限代码。

当前注册接口已经按这个规则实现：已有用户后注册新用户会在 route layer 重新查询数据库当前权限，撤销用户角色后，旧 token 不能继续注册用户。

用户管理接口也按这个规则实现：拥有 `user.manage` 的用户可以查询用户、查询角色/权限、启停账号、整体替换用户角色和重置用户密码。禁用用户后，该用户已有 access token 和 refresh token 都会因为数据库中的用户状态不是 `active` 而被拒绝。

为避免系统锁死，用户管理接口禁止禁用最后一个 active admin，也禁止从最后一个 active admin 身上移除 `admin` 角色。这里的 `admin` 仍然只是角色模板；业务授权继续判断 `user.manage` 等权限代码。

## 新增受保护能力

新增业务接口或能力时按以下步骤处理：

1. 在对应领域的权限常量文件中定义稳定权限代码，命名使用 `domain.action` 形式，例如 `stock.write`；当前用户域使用 `core/src/users/permissions.rs`，库存和审计权限使用 `core/src/stock/permissions.rs`。
2. 在 `core/src/rbac/bootstrap.rs` 的内置权限中补齐权限说明，并把权限分配给合适的内置角色模板。
3. 在对应业务模块的 `mod.rs` 路由装配处挂载权限 middleware，不在 handler 中判断角色或权限代码。
4. 如果权限要求取决于运行时条件，复用条件鉴权封装，把条件函数和权限代码作为路由参数组合。
5. 对需要立即响应授权变更的管理能力，校验 token 后重新读取数据库当前权限。
6. 更新 OpenAPI、数据库文档和测试。

## 禁止做法

- 不要用角色代码作为业务授权条件。
- 不要实现角色等级比较。
- 不要把 `admin` 写成隐含超级权限绕过。
- 不要在 handler 中直接拼接 RBAC 表关联 SQL。
- 不要在启动 RBAC 初始化中创建用户。
- 不要在 JSON 启动配置里定义角色、权限或 JWT signing key。
