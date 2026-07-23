# Core HTTP、鉴权与用户代码地图

本文覆盖 `core/src/http`、`security`、`auth`、`users` 和 `rbac`。

## 全局 HTTP 外壳

- `core/src/http/cors.rs`：统一 CORS 响应头和 OPTIONS 预检，不拥有前端资源；middleware 在所有业务 Router merge 完成后挂载，确保 auth、users 和 stock 路由都被覆盖。
- `core/src/http/docs.rs`：Debug OpenAPI 路径、元信息和业务 tag；启用 Swagger UI feature 时挂载开发期 UI，Release 不注册文档路由并避免编译 Swagger UI。
- `core/src/http/error_response.rs`：统一 `{ error: { code, message, details } }` 非 2xx JSON 契约及 404/405。
- `core/src/http/health.rs`：无状态 `/api/health`。
- `core/src/http/router.rs`：组装文档、健康检查、auth、users 和 stock Router，并注入 `CoreState`。
- `core/src/http/validation.rs`：`ValidatedJson`、`ValidatedPath`、`ValidatedQuery`；字段校验失败返回结构化 `details.fields`。

## Security 前置层

- `core/src/security/current_user.rs`：`CurrentUser` extractor 和 bearer token 解析。
- `core/src/security/jwt.rs`：`SecurityRuntime`、JWT claims 和 access token 签发/校验。
- `core/src/security/middleware.rs`：路由鉴权中间件和 `AuthorizeRouteExt`；进入业务 handler 前读取数据库当前权限，支持单权限、“任一权限”及首用户注册条件策略。
- `core/src/security/password.rs`：Argon2 密码哈希与校验。
- `core/src/security/token.rs`：refresh token 哈希、随机文本和 JWT 时间戳。
- `core/src/security/error.rs`：security、auth、users 共用的鉴权错误映射。

强制改密用户只允许访问 `/api/auth/me` 和 `/api/auth/me/password`，其它已鉴权接口返回 `password_change_required`。

## Auth 会话模块

- `core/src/auth/mod.rs`：注册 bootstrap status、login、refresh 和 logout 路由。
- `core/src/auth/contract.rs`：首用户初始化状态、登录、刷新、登出、用户摘要、token 响应和客户端类型 DTO。
- `core/src/auth/controller.rs`：鉴权 HTTP 入口和 OpenAPI 标注。
- `core/src/auth/service.rs`：登录、refresh token 轮换、复用检测和登出吊销。
- `core/src/auth/bootstrap.rs`：数据库托管的鉴权设置、active signing key 和启动结果。

## Users 用户模块

- `core/src/users/mod.rs`：注册、当前用户、改密、用户管理和权限定义路由及权限声明。
- `core/src/users/controller.rs`：用户 HTTP DTO、handler 和 OpenAPI 标注。
- `core/src/users/service.rs`：用户业务服务入口和子模块重新导出。
- `core/src/users/service/register.rs`：用户注册、首个用户事务判断、全部权限分配和审计；并发初始化失败返回稳定 `initial_user_already_exists`。
- `core/src/users/service/me.rs`：当前用户快照与自助改密。
- `core/src/users/service/management.rs`：用户列表、详情、启停、软删除、权限替换、临时密码、防锁死规则，以及当前操作者关键权限保护。
- 用户管理 service 同时拒绝当前操作者停用或软删除自己，也拒绝为自己设置管理员临时密码，避免绕过前端入口限制。
- `auth_users.deleted_at` 由初始 schema 直接创建；用户仓储默认排除软删除记录，删除事务同时停用账号、吊销 refresh token 并写审计。
- 当前操作者可以调整自己的其他权限，但不能增加或移除自己的 `user.permissions.update` 和 `user.permission.read`。
- `core/src/users/service/response.rs`：用户与权限响应投影。
- `core/src/users/service/pagination.rs`：用户分页。
- `core/src/users/service/validation.rs`：用户名、搜索、状态和权限代码归一化。
- `core/src/users/permissions.rs`：用户域稳定权限代码。

## RBAC

- `core/src/rbac/bootstrap.rs`：内置用户、库存和审计权限定义的幂等补齐。
- 当前授权模型是“用户直接拥有权限”，不创建角色或角色权限关系。
- 首个用户分配全部内置权限由用户注册事务完成，不由 RBAC bootstrap 创建用户。
