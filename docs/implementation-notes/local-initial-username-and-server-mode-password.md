# 本机用户初始化与 server-mode 首用户分流重设计方案

> 日期：2026-08-03
>
> 状态：已实施；core 目标测试、OpenAPI 类型生成和 frontend 构建已通过，桌面/Android 实机 smoke 待执行。
>
> 本方案替换本文件上一版“在初始化向导中增加用户名步骤”的设计。核心调整是：初始化向导只负责运行方式，账号初始化统一复用认证层的 `/register` 页面，避免本机用户名页面与 server-mode 的注册页面处于不同交互层级。

## 1. 设计原则

WineStock 的首次启动应分成两个层级：

```text
/setup       运行配置层：选择本机、server-mode 或远端连接
    -> /auth 认证层：根据服务是否已有用户进入登录、注册或本机用户初始化
```

账号创建不放进 `/setup`。因此：

- `server-mode` 或远端服务没有用户时，继续使用现有 `/auth -> /register` 分流；
- `self-hosted` 本机服务没有用户时，进入同一个 `/register` 页面，但使用“本机初始化”模式，只显示用户名；
- `/setup` 只改变运行模式，不收集用户名、密码或其它用户业务字段。

这样本机模式和 server-mode 首用户注册处于同一个认证页面层级，只是提交动作和密码呈现方式不同。

## 2. 当前机制与问题

当前已有以下机制：

1. 初始化向导负责运行配置并在 apply 成功后进入 `/auth`。
2. `/auth` 查询 `GET /api/auth/bootstrap-status`；服务没有用户时已有用户会自动分流到 `/register`。
3. `self-hosted` 首次 `POST /api/auth/local-session` 换取本机会话时，core 惰性创建用户输入的用户名，密码为随机占位密码。
4. 切换 `server-mode` 时，运行设置页只要求设置新密码，并把密码提交到 `POST /api/auth/me/password`。

问题有两个：

- 如果把本机用户名加到 `/setup`，它会与 server-mode 的 `/register` 处于不同层级，首次账号体验不一致；
- 密码接口由 token 已经确定当前用户，却仍允许请求体提交用户名，导致 server-mode 设密流程同时承担账号改名。

相关现有入口：

- `frontend/src/pages/SetupWizardPage.vue`：运行配置向导；
- `frontend/src/pages/AuthEntryPage.vue`、`frontend/src/router/guards.ts`：认证入口和首用户分流；
- `frontend/docs/implementation-notes/initial-user-bootstrap-routing.md`：无用户自动进入注册页的既有方案；
- `core/src/users/service/local_admin.rs`：按 `initial_username` 创建本机静默用户；
- `frontend/src/pages/RuntimeSettingsPage.vue`：server-mode 设密门；
- `core/src/users/controller.rs`、`core/src/users/service/me.rs`：当前用户改密契约与实现。

## 3. 目标页面结构

### 3.1 运行配置层 `/setup`

初始化向导只呈现运行方式和设备偏好：

| 平台 | `/setup` 可选运行方式 | 说明 |
| --- | --- | --- |
| Desktop | `self-hosted`、`server-mode`、连接已有服务器 | `server-mode` 仅在 `capabilities.serverMode = true` 时显示 |
| Android | `self-hosted`、连接已有服务器 | 继续禁用 `server-mode` |
| Web fallback | 连接已有服务器 | 浏览器不启动本地服务 |

Desktop 的 `server-mode` 选择沿用现有 Shell capability，不新增平台判断或配置字段。端口和监听地址继续由 Shell 默认配置及运行设置高级配置管理。用户名不进入 `EditableRuntimeConfig`、Shell Bridge 快照或配置文件。

`/setup` apply 成功后，无论选择哪种模式，都进入统一认证入口 `/auth`。向导不因为本机模式而增加用户名步骤。

### 3.2 认证层 `/auth`

`/auth` 根据运行模式、服务是否已有用户和当前会话状态选择认证分支：

```text
/auth
  -> 已有会话                         -> 进入业务页面
  -> server-mode/远端 + 无用户         -> 现有 /register（普通注册模式）
  -> self-hosted 本机 + 无用户         -> 现有 /register（本机初始化模式）
  -> 已有用户但未登录                  -> 现有登录状态
```

“无用户自动跳转注册页”是当前已有机制，本方案不修改它。新增的只有本机 `self-hosted` 无用户分支；该分支仍由 `/auth` 认证层承载，视觉和页面层级与注册流程保持一致。

建议让 `AuthEntryPage` 继续作为状态编排入口：

- `needs-initial-user`：沿用现有逻辑跳转 `/register`，用于 server-mode 和远端，显示用户名、密码和确认密码；
- `requires_initial_user = true`：继续跳转 `/register`；`RegisterPage.vue` 根据 self-hosted 本机运行快照进入本机初始化模式，只显示用户名和创建按钮；
- `login`：沿用现有登录流程；
- `checking`、`error`：沿用现有状态和恢复操作。

本机初始化应复用 `RegisterPage.vue` 的页面外壳、账户语义、返回行为、校验和错误反馈，但切换为本机初始化模式：用户名仍由用户明确输入，密码字段不渲染、不生成前端假密码，也不发送隐藏密码。core 在服务端生成随机占位密码并只保存其哈希，因此业务上已经完成密码设置，但密码不会暴露给用户；之后切换 `server-mode` 时再设置用户可知的真实密码。

## 4. 三条首次启动流程

### 4.1 Desktop `server-mode`：正常注册

```text
/setup
  -> 选择 server-mode
  -> apply 运行配置
  -> /auth
  -> bootstrap-status.requires_initial_user = true
  -> 现有 /register
  -> 输入用户名 + 密码
  -> 创建真实首用户并进入应用
```

server-mode 不生成本机静默换取凭据，不创建随机占位密码用户，也不使用本机初始化字段。用户在注册页一次性完成用户名和真实密码设置。

### 4.2 Desktop/Android `self-hosted`：认证层本机初始化

```text
/setup
  -> 选择 self-hosted
  -> apply 运行配置
  -> /auth
  -> 检测服务无用户且存在可信本机 Shell 能力
  -> 现有 /register（本机初始化模式）
  -> 输入用户名
  -> local-session 携带用户名完成首次创建和静默登录
  -> 进入应用
```

`RegisterPage.vue` 的本机初始化模式只显示用户名：

- 复用注册用户名的去首尾空白、非空、长度和文本校验；
- 不显示密码和确认密码；密码由 core 生成随机占位值并保存哈希；
- 不提供 `admin` 默认值，也不自动填充用户名；
- 用户名只在本次初始化请求中使用，不写入运行配置；
- 提交失败时保留用户名并显示可重试状态。

core 仍创建随机占位密码和全部内置权限。之后只有切换 `server-mode` 时才要求设置真实密码。

### 4.3 远端连接：正常注册或登录

```text
/setup
  -> 连接已有服务器
  -> apply 远端地址
  -> /auth
  -> 无用户：现有 /register
  -> 有用户：现有登录
```

远端服务不使用本机账户初始化状态，也不接受本机 Shell 的换取凭据。

## 5. 本机首次建户契约

现有 `POST /api/auth/local-session` 继续作为本机静默会话入口，但增加“空库必须先提供用户名”的语义：

```json
{
  "exchange_token": "...",
  "device_name": "...",
  "client_kind": "desktop",
  "version": "...",
  "initial_username": "alice"
}
```

`initial_username` 只用于空用户库的首次本机建户，不是运行配置字段，也不是密码修改接口的用户名字段。core 规则如下：

1. 先校验壳内换取凭据，再进入与普通首用户注册共用的写锁和事务。
2. 已有 `local_auto_login_user_id` 标记时，直接返回标记用户，忽略 `initial_username`，绝不通过本接口改名。
3. 没有标记但已有其它用户时，继续返回 `local_session_unavailable`，不启发式绑定任意用户。
4. 空用户库且没有 `initial_username` 时，返回稳定错误 `local_initial_user_required`，不创建 `admin` 兜底用户。
5. 空用户库且用户名有效时，使用归一化后的用户名创建用户，授予全部内置权限，写入用户 ID 标记和占位密码标记，再签发正常 token。

前端会话层把 `local_initial_user_required` 视为匿名状态，使路由进入 `/auth`；`AuthEntryPage` 沿用现有 bootstrap 分流跳转 `/register`，再由 `RegisterPage.vue` 根据有效 self-hosted 本机运行快照进入本机初始化模式，而不是服务不可用，也不能直接回落到普通登录页。用户名由本机表单持有，不需要写入 `sessionStorage`；页面重载后重新显示该表单即可。

首次创建与普通首用户注册继续共用写锁，因此并发情况下只能创建一个用户。已经创建标记用户后，重复请求中的用户名不会覆盖既有用户名。

## 6. server-mode 设密契约

本机用户切换到 `server-mode` 时，设密门只设置密码，不再修改用户名。

### 6.1 请求体

`POST /api/auth/me/password` 从：

```json
{
  "username": "alice",
  "current_password": "",
  "new_password": "new-password"
}
```

调整为：

```json
{
  "current_password": "",
  "new_password": "new-password"
}
```

规则：

- 当前用户完全由 Bearer token 的 `CurrentUser.user_id` 决定；
- 普通改密仍要求 `current_password`；
- 本机标记用户仍处于随机占位密码状态时允许 `current_password` 为空；
- 成功后清除 `local_auto_login_password_placeholder`；
- 只更新密码哈希、更新时间和占位标记，不更新用户名；
- 审计只记录密码字段和模式，不再产生 `username_and_password`；
- 保持 `serde(deny_unknown_fields)`，旧调用方发送 `username` 时应得到契约错误。

service 层应使用只更新密码的 repository 操作。如果现有 `update_credentials` 同时更新用户名，应增加明确的 `update_password` 操作，避免通过传回原用户名保留旧职责。

### 6.2 前端设密门

`RuntimeSettingsPage.vue` 的 Dialog：

- 可以只读显示当前账号用户名，作为用户确认上下文；
- 不显示可编辑用户名输入；
- 不添加隐藏用户名字段；
- 请求只发送 `current_password` 和 `new_password`；占位态仍允许当前密码为空；
- 设置成功后刷新 `/api/auth/me` 仅用于同步展示，不把用户名作为请求参数；
- 密码成功后再继续原有 server-mode 确认、端口和防火墙流程。

用户名后续修改仍只通过既有 `PATCH /api/users/{id}/username`，不与当前用户改密接口合并。

## 7. 分组件影响

### core

- 删除固定 `LOCAL_ADMIN_USERNAME`，将 `provision_local_admin` 改为不含管理员假设的本机首用户创建逻辑；
- `local-session` 空库无用户名时返回 `local_initial_user_required`，不创建默认用户；
- 增加并校验 `initial_username`，保留用户 ID 标记和占位密码标记，不改数据库 schema；
- 删除 `UserPasswordChangeRequest.username`，当前用户改密只更新密码；
- 增加只更新密码的 repository/service 操作；
- 更新 OpenAPI、稳定错误码、审计详情和 core 测试；
- 已确认首用户初始化状态仍被 Server 和 Android native 消费，因此统一使用 `initial_user_setup_required`；本字段仍只表示数据库是否为空。

### frontend

- `SetupWizardPage.vue` 只增加 Desktop `server-mode` 运行方式选择，不增加用户名步骤；
- `AuthEntryPage.vue` 保留现有无用户跳转 `/register`；`RegisterPage.vue` 根据有效 self-hosted 运行模式进入本机初始化变体；
- `auth/session.ts` 识别 `local_initial_user_required`，提供带用户名的首次本机换取操作；
- 保留现有 server-mode/远端 `/auth -> /register` 机制，只增加 Desktop server-mode 首次选择的回归覆盖；
- `RuntimeSettingsPage.vue` 删除设密用户名输入和请求字段；
- `ChangePasswordPage.vue` 删除隐藏用户名输入和调用参数；
- `pnpm gen:api` 同步 `AuthLocalSessionRequest` 和 `UserPasswordChangeRequest` 类型。

### Desktop、Android、Server 和 shared

- Desktop 只在 `capabilities.serverMode = true` 时显示 server-mode；首次 server-mode 不生成本机静默凭据；
- Android 继续禁用 server-mode；首次 self-hosted 进入认证层本机用户名状态；
- Web fallback 只连接远端，继续使用普通注册/登录；
- Server Shell 不新增用户名启动参数，空库继续通过普通首用户注册初始化；
- Shell Bridge 继续只传递运行配置、服务状态和本机换取凭据，不承载用户名业务字段；用户名通过 core HTTP 请求传递；
- shared 不增加用户名配置键，不改变运行模式、端口和监听地址规则。

## 8. 存量数据与兼容

不需要数据库 schema 迁移。`auth_users.username` 已存在，`local_auto_login_user_id` 继续按用户 ID 绑定。

- 已有本机标记用户（包括 `admin`）保持原用户名、用户 ID、权限和业务关联，不升级时强制改名；
- 已有占位密码标记继续在切换 server-mode 时要求只设置密码；
- 已有真实用户但没有本机标记时，继续拒绝本机静默换取，不自动绑定唯一用户；
- 新空本机库必须先在认证层输入用户名，不再自动创建 `admin`；
- 新空 server-mode 库继续走现有 `/register`，由用户设置用户名和真实密码；
- 新 core 与旧前端需要同版本交付：旧前端在空本机库中不会提供本机初始化变体，旧改密请求携带 `username` 也会被新契约拒绝；
- 不保留静默吞掉旧 `username` 字段的兼容层，避免再次允许密码接口隐式改名。

## 9. 验收矩阵

### 认证分流

- Desktop `/setup` 显示 self-hosted、server-mode 和远端；server-mode 受 capability 控制；
- Android/Web 不显示或接受 server-mode；
- server-mode 无用户时，apply 后沿用现有 `/auth -> /register`；
- 远端无用户时，沿用现有 `/auth -> /register`；
- self-hosted 无用户时，`/auth` 跳转现有 `/register` 的本机初始化变体，不进入死循环登录；
- 已有本机标记用户时直接静默登录，不重复询问用户名。

### Core 与账户

- 空 self-hosted 库首次带 `initial_username = alice` 创建 `alice`，不创建 `admin`；
- 空 self-hosted 库缺少或输入非法用户名时返回 `local_initial_user_required`/校验错误，数据库不产生用户；
- 并发首次本机初始化只能创建一个用户；
- 已有标记用户携带不同用户名时不改名；
- server-mode 空库不会调用 local-session 自动创建用户；
- `POST /api/auth/me/password` 不带用户名可以完成普通改密和占位态设密；
- 旧用户名字段被 `deny_unknown_fields` 拒绝；
- 用户管理改名接口仍独立工作，权限、审计和用户 ID 关联不回退。

### UI 与平台

- 本机用户名页面与注册页都属于认证层，页面层级和返回/错误处理一致；
- 本机用户名页面只有用户名输入，不要求密码；
- server-mode 设密 Dialog 不显示可编辑用户名，不发送隐藏用户名；
- Desktop self-hosted -> server-mode 后可以用初始化时的用户名和新密码登录；
- Desktop 首次 server-mode 注册时用户名和密码均由 `/register` 完成；
- Android Activity 重建不会重复创建或覆盖本机用户；
- 验证 `1440 x 900`、接近 `768px` 和 `390 x 844`，无横向溢出、控制台无新增错误或 Vue 警告。

## 10. 实施顺序

1. 修改 core 的本机首次换取、默认用户名删除、稳定错误码和只更新密码契约，补齐 core 测试。
2. 读取运行中的 `/api-docs/openapi.json`，确认 `AuthLocalSessionRequest` 和 `UserPasswordChangeRequest` 契约符合方案。
3. 修改 frontend 认证入口，复用 `/register` 的本机用户名表单变体；不把用户名加入 `SetupWizardPage`。
4. 在 `SetupWizardPage` 增加 Desktop capability 控制的 server-mode 选择，并验证 apply 后仍进入统一 `/auth`。
5. 修改 server-mode 设密门和普通改密页面，运行 `cd frontend && pnpm gen:api` 同步生成类型。
6. 同步 `self-hosted-silent-auth.md`、`core/docs/user-management-api.md`、`frontend/docs/page-runtime-settings.md`、认证入口文档和相关代码地图/源码头注释。
7. 按 Desktop self-hosted、Desktop server-mode、Android self-hosted、远端空库和 Server Shell 空库矩阵执行 smoke；确认现有用户数据未被改名或重绑。

## 11. 风险与处理

| 风险 | 处理决定 |
| --- | --- |
| 本机初始化与普通注册体验不一致 | 两者复用同一个 `/register` 页面；差异只在本机模式隐藏密码输入，由 core 设置随机占位密码。 |
| 空本机库被旧自动换取逻辑抢先创建 | core 先返回 `local_initial_user_required`；frontend 把该错误映射为本机初始化状态，不允许无用户名自动建户。 |
| 本机用户未设置真实密码就开放 LAN | 保留 `password_placeholder` 检查和 server-mode 强制设密门。 |
| 用户名被密码请求体重新指定 | 删除 `UserPasswordChangeRequest.username`，身份只取 token user ID；改名使用独立用户管理 API。 |
| server-mode 首次注册与本机静默创建混淆 | server-mode 不生成 local-session 凭据，始终走现有 `/register`；本机初始化只在 self-hosted 且有可信 Shell 时启用。 |
| 新旧前端契约不一致 | core、frontend、Desktop/Android 资源作为同一版本交付，不静默兼容旧字段。 |

## 12. 完成标准

实施结果满足：`/setup` 只负责运行方式；Desktop 首次 `server-mode` 无用户沿用现有 `/auth -> /register`；本机无用户在同一认证层复用 `/register` 显示用户名初始化变体并按用户名创建，不再生成默认用户；切换 server-mode 的密码请求不包含用户名且不能改名；已有用户、权限、业务关联、审计用户 ID 和模式切换恢复行为不回退。
