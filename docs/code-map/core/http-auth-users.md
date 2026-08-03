# Core HTTP、鉴权与用户代码地图

本文覆盖 `core/src/http`、`security`、`auth`、`users` 和 `rbac` 的所有权与边界。
逐文件职责以源码中文文件头注释为准。

## 全局 HTTP 外壳（`core/src/http/`）

- 组装文档、健康检查、auth、users 和 stock Router 并注入 `CoreState`；CORS middleware 在全部业务 Router merge 完成后挂载，确保各域路由都被覆盖。
- 统一 `{ error: { code, message, details } }` 非 2xx JSON 契约与 404/405；`ValidatedJson`/`ValidatedPath`/`ValidatedQuery` 校验失败返回结构化 `details.fields`。
- Debug 构建注册 OpenAPI 元信息，启用 feature 时挂载 Swagger UI，并公开 `openapi_document_json()` 供开发期契约导出（`core/examples/dump_openapi.rs`）；Release 不注册文档路由、不编译 Swagger UI 与导出函数。
- 无状态 `/api/health` 供服务可用性探测。

## Security 前置层（`core/src/security/`）

- `CurrentUser` extractor 与 bearer token 解析、`SecurityRuntime` JWT 签发/校验、Argon2 密码哈希、refresh token 哈希与随机文本，以及 security/auth/users 共用的鉴权错误映射。
- 路由鉴权中间件（`AuthorizeRouteExt`）在进入业务 handler 前读取数据库当前权限，支持单权限、“任一权限”及首用户注册条件策略。
- 强制改密用户只允许访问 `/api/auth/me` 和 `/api/auth/me/password`，其它已鉴权接口返回 `password_change_required`。

## Auth 会话模块（`core/src/auth/`）

- bootstrap status、login、local-session、refresh、logout 路由、DTO 与 OpenAPI 标注；service 负责登录、refresh token 轮换、复用检测和登出吊销。
- `bootstrap.rs`：数据库托管的鉴权设置、active signing key 和启动结果。
- 本机静默会话（`/api/auth/local-session`）：仅 `self-hosted` 模式生成 per-boot 换取凭据（`bootstrap.rs` 的 `LocalSessionSecret`，经 `LocalServiceInfo` 进程内交给壳），端点匿名但凭据比较采用 SHA-256 摘要；成功后与登录共用 token 签发路径。`/local-session/status` 已鉴权，返回标记用户的占位密码状态，供前端切 `server-mode` 前的设密门使用。设计见 `docs/implementation-notes/self-hosted-silent-auth.md`。

## Users 用户模块（`core/src/users/`）

- 注册、当前用户、改密、用户管理和权限定义的路由、DTO 与服务；`service/` 按注册、当前用户、管理、投影、分页和归一化拆分。当前改密只更新 Bearer token 对应用户的密码；用户管理改名使用独立的 `PATCH /api/users/{id}/username`。
- `service/local_admin.rs`：本机免登录标记用户的惰性开通（空库首次换取时按 `initial_username` 创建用户，随机占位密码 + 全部内置权限，与首用户注册共用写锁）、自愈（停用/软删除/收权恢复并写审计）与占位密码标记（鉴权设置行 `local_auto_login_user_id` / `local_auto_login_password_placeholder`）；占位态允许免旧密码改密，任何针对标记用户的改密/重置都会清除占位标记。
- 关键规则：首用户注册在事务内完成全部权限分配，并发初始化失败返回稳定 `initial_user_already_exists`；当前操作者不能停用或软删除自己、不能为自己设置临时密码、可调整自己其他权限但不能增减自己的 `user.permissions.update` 与 `user.permission.read`。
- `auth_users.deleted_at` 由初始 schema 直接创建；用户仓储默认排除软删除记录，删除事务同时停用账号、吊销 refresh token 并写审计。
- `permissions.rs`：用户域稳定权限代码。

## RBAC（`core/src/rbac/`）

- 内置用户、库存和审计权限定义的幂等补齐。
- 当前授权模型是“用户直接拥有权限”，不创建角色或角色权限关系；首个用户的全部权限分配由注册事务完成，不由 RBAC bootstrap 创建用户。
