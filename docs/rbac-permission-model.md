# RBAC 权限模型

本文档定义 WineStock 当前 RBAC 模型、启动初始化行为和业务授权规则。

实现以 `core/src/rbac.rs`、`core/src/auth.rs`、`core/src/persistence/repository/rbac.rs` 和数据库 migration 为准。

## 核心规则

业务授权统一判断权限代码，不判断角色代码。

角色只负责批量授予权限。角色不是业务授权等级，也不表达 `admin > staff > viewer` 这类层级关系。业务代码不得写“只要是 `admin` 角色就允许”的判断；需要受保护的能力必须定义明确权限，例如 `user.register` 或 `stock.write`。

## 职责边界

`core/src/rbac.rs` 负责：

- 定义内置角色。
- 定义内置权限。
- 定义内置角色包含哪些权限。
- 在本地服务启动时补齐缺失的 RBAC 基础数据。

`core/src/persistence/repository/rbac.rs` 负责：

- 查询用户经由角色获得的权限代码。
- 查询用户直接分配到的角色代码。
- 补齐角色、权限和分配关系。
- 隔离 RBAC 表结构，避免 handler 直接拼接关联查询。

`core/src/auth.rs` 负责：

- 登录、注册、刷新、登出和当前用户接口。
- JWT access token 签发和校验。
- 在需要授权的业务入口检查权限代码。

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

内置角色权限关系：

| 角色代码 | 权限 |
| --- | --- |
| `admin` | `user.register`, `user.manage`, `stock.read`, `stock.write` |
| `staff` | `stock.read`, `stock.write` |
| `viewer` | `stock.read` |

## 首个用户

当数据库没有任何用户时，`POST /api/auth/register` 不要求 Bearer token。

首个注册用户会自动分配 `admin` 角色。它不是因为 `admin` 角色本身拥有特殊授权等级，而是因为 `admin` 角色模板包含当前全部内置权限。

当数据库已经存在用户后，注册新用户必须由当前拥有 `user.register` 权限的 Bearer token 调用。

## JWT 与当前授权

登录和刷新时，access token 会携带签发时的 `roles` 和 `permissions` 快照。响应体也会返回当前用户的角色和权限列表，便于客户端展示。

需要当前授权状态的管理接口不能只依赖 JWT 快照。它应在校验 Bearer token 后重新读取数据库中的当前权限，再判断具体权限代码。

当前注册接口已经按这个规则实现：已有用户后注册新用户会重新查询数据库当前权限，撤销用户角色后，旧 token 不能继续注册用户。

## 新增受保护能力

新增业务接口或能力时按以下步骤处理：

1. 定义稳定权限代码，命名使用 `domain.action` 形式，例如 `stock.write`。
2. 在 `core/src/rbac.rs` 的内置权限中补齐权限说明。
3. 把权限分配给合适的内置角色模板。
4. 在 handler 或服务层检查权限代码，不检查角色代码。
5. 对需要立即响应授权变更的管理能力，校验 token 后重新读取数据库当前权限。
6. 更新 OpenAPI、数据库文档和测试。

## 禁止做法

- 不要用角色代码作为业务授权条件。
- 不要实现角色等级比较。
- 不要把 `admin` 写成隐含超级权限绕过。
- 不要在 handler 中直接拼接 RBAC 表关联 SQL。
- 不要在启动 RBAC 初始化中创建用户。
- 不要在 JSON 启动配置里定义角色、权限或 JWT signing key。
