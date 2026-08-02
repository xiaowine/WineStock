# 用户名修改与 server-mode 设密整改方案

> 状态：已实施并完成最终回归验证。
>
> 本文记录本机 `self-hosted` 切换到 `server-mode` 时的账号设密调整，以及用户管理页面的独立用户名修改能力。本文是跨 `core`、`frontend` 和 Desktop 运行设置流程的实施依据。

## 1. 定稿结论

系统中的业务数据通过用户 ID 关联，不通过用户名关联。因此修改用户名不需要迁移库存、单据、权限、文件所有者、审计关联或 refresh token 数据。

采用两条不同的接口路径：

1. `self-hosted` 切换 `server-mode`：继续使用现有的 `POST /api/auth/me/password`，在原请求中加入必填 `username` 字段；不新增第二个“设密”接口。
2. 用户管理页面修改任意用户用户名：新增独立的 `PATCH /api/users/{id}/username` 接口；不复用当前用户改密接口。

`POST /api/auth/me/password` 的 `username` 不允许省略。所有前端调用方都必须传入用户名：普通修改密码时传入当前用户名，切换 `server-mode` 的设密流程传入用户确认后的新用户名。

## 2. 当前数据与会话边界

用户名只在 `auth_users.username` 中作为唯一登录标识保存。以下数据使用用户 ID：

- JWT 的 `sub`；
- `auth_refresh_tokens.user_id`；
- 用户权限分配表的 `user_id`；
- 文件所有者 `owner_user_id`；
- 单据创建人、审批人和拒绝人字段；
- 审计事件 `audit_events.user_id`；
- 本机静默登录标记 `local_auto_login_user_id`。

因此用户名修改后的行为为：

- 用户 ID、权限、业务历史和本机静默登录目标不变；
- 旧用户名立即不能用于登录，新用户名生效；
- 已有 access token 和 refresh token 不因用户名变化失效；
- `/api/auth/me` 按用户 ID 查询，会返回修改后的用户名；
- 创建用户或历史审计详情中保留的旧用户名属于历史快照，不回写为新用户名；
- 软删除用户占用的用户名继续保留，不能被其它账号复用。

## 3. server-mode 切换设密

### 3.1 请求契约

将现有 `UserPasswordChangeRequest` 从：

```json
{
  "current_password": "",
  "new_password": "new-password"
}
```

调整为：

```json
{
  "username": "new-admin",
  "current_password": "",
  "new_password": "new-password"
}
```

字段规则：

| 字段               | 要求                                                     |
| ------------------ | -------------------------------------------------------- |
| `username`         | 必填；复用注册用户名的去首尾空白、非空、长度和唯一性校验 |
| `current_password` | 普通改密必须提供；本机占位密码状态允许为空               |
| `new_password`     | 必填；复用当前改密密码规则                               |

普通用户修改密码时，前端从当前会话快照读取用户名并原样传入。只有切换 `server-mode` 的设密对话框提供修改用户名的交互。

### 3.2 core 处理

`POST /api/auth/me/password` 必须在同一个数据库事务中完成：

1. 按当前 token 的用户 ID读取用户；
2. 校验新用户名并检查唯一性，排除当前用户自身；
3. 按现有规则校验当前密码；占位密码状态继续允许当前密码为空；
4. 更新 `auth_users.username`、密码哈希和 `updated_at`；
5. 成功设置真实密码后清除本机占位密码标记；
6. 写入用户更新审计；
7. 整体提交，任一步失败都不更新用户名或密码。

用户名没有变化时仍执行相同的必填校验，但不产生用户名变更内容。审计详情不得包含明文密码、密码哈希、access token 或 refresh token；用户名变更可记录旧值和新值。

### 3.3 前端流程

运行设置页检测到 `password_placeholder = true` 时，打开“设置当前用户账号”流程，表单同时包含：

- 用户名，默认填充当前用户名 `admin`，允许修改；
- 新密码；
- 新密码确认。

提交顺序保持不变：先调用 `POST /api/auth/me/password`，成功后再应用 `server-mode` 运行配置。若防火墙授权、端口或服务启动失败，账号修改仍然保留，用户可以重试运行配置；不得回滚已经成功的账号凭据修改。

设密成功后前端必须重新请求 `/api/auth/me`，更新内存中的用户名快照，再继续运行模式切换。当前会话不需要退出或重新登录。

## 4. 用户管理用户名修改

### 4.1 接口

新增：

```text
PATCH /api/users/{id}/username
```

请求：

```json
{
  "username": "new-name"
}
```

响应复用 `UserAdminResponse`，返回修改后的用户名和当前用户管理快照。

使用独立权限 `user.username.update`，因为用户名是登录身份标识，不应隐含在密码重置或权限修改权限中。后端重新读取当前权限并执行授权，前端隐藏入口不能作为安全边界。

存量用户只会在启动时补齐权限定义，不会被静默授予新权限。server-mode 下由已有权限管理员在用户权限界面显式授予 `user.username.update`；self-hosted 的本机静默目标用户仍沿用既有自愈规则补齐全部内置权限。

### 4.2 修改规则

- 只能修改未软删除用户；
- 用户名必须经过与注册一致的归一化和唯一性检查；
- 修改其它用户用户名不吊销其现有会话，用户 ID 未变化；
- 修改当前用户的用户名不改变当前 access token、refresh token 或权限；
- 修改成功后用户列表、详情和当前用户快照使用新用户名；
- 写入 `entity_type = "user"`、`action = "updated"` 的审计事件，记录字段名及旧/新用户名；
- 不允许把软删除账号占用的用户名重新分配给其它账号。

### 4.3 用户管理界面

用户管理页面在现有用户操作中增加“修改用户名”入口，不与“设置临时密码”合并。建议新增 `UserUsernameDialog.vue`，沿用现有账号上下文、表单校验和全局 Notice 反馈。

成功后：

- 更新当前列表中的目标用户记录；
- 如果目标用户是当前登录用户，重新获取 `/api/auth/me` 更新会话展示；
- 不刷新或吊销其它会话；
- `username_taken`、校验失败和权限不足统一通过 Notice 呈现，不增加固定错误文本挤压 Dialog。

## 5. 契约与代码影响

### core

- 扩展 `UserPasswordChangeRequest`，将 `username` 设为必填；
- 在当前用户改密 service 中加入用户名归一化、唯一性检查和同事务更新；
- 新增 `UserUsernameUpdateRequest`、controller、service 和 repository 更新方法；
- 新增 `user.username.update` 内置权限并纳入权限初始化；
- 扩展 `AuthApiError` 的用户名冲突、无权限和目标用户不存在路径复用；
- 增加用户名修改审计；
- 增加 current-user、placeholder 用户名变更、重复用户名、事务失败和用户管理权限测试。

### frontend

- 执行 `cd frontend && pnpm gen:api`，同步必填 `username` 和新接口类型；
- 更新 `frontend/src/api/auth.ts` 的改密调用类型和所有调用方；
- 更新 `RuntimeSettingsPage.vue` 的设密对话框，提交用户名和密码；
- 更新 `ChangePasswordPage.vue`，始终传入当前用户名；
- 更新 `frontend/src/api/users.ts`、`UsersPage.vue` 和用户管理组件；
- 用户名冲突、字段校验和请求失败统一使用 Notice，不新增固定错误文本区域。

### 文档

已同步更新：

- `core/docs/user-management-api.md`；
- `frontend/docs/user-management.md`；
- `frontend/docs/page-runtime-settings.md`；
- `docs/implementation-notes/self-hosted-silent-auth.md` 中的 server-mode 设密流程。

## 6. 验收矩阵

### server-mode 设密

- 占位密码状态下输入新用户名和新密码，用户名与密码同时成功更新；
- 占位密码状态下用户名为空、重复或非法时，用户名和密码均不发生变化；
- 普通改密请求缺少 `username` 时返回校验错误；
- 普通改密传入当前用户名时行为与现有流程一致；
- 设密成功后继续切换 `server-mode`，当前会话保持有效；
- 防火墙/UAC 或服务启动失败时，已成功修改的用户名和密码仍保留；
- 新用户名可以登录，旧用户名不能登录；
- 业务记录、权限、文件所有者、单据审批人和审计用户 ID 不变化。

### 用户管理改名

- 具备 `user.username.update` 权限的用户可以修改其它用户用户名；
- 无该权限时前端不显示入口，后端直接拒绝请求；
- 重复用户名、软删除用户名和非法用户名均被拒绝；
- 修改其它用户后，其已有会话仍然有效，新用户名登录成功；
- 修改当前用户后，当前会话和权限不丢失，界面展示更新；
- 用户列表搜索和用户详情使用新用户名；
- 审计记录包含修改前后用户名，但不包含任何密码或 token；
- 桌面端和 390px 移动视口下 Dialog 无横向溢出，错误统一通过 Notice 展示。

## 7. 非目标

- 不修改任何业务表的用户关联字段；
- 不把用户名写入 JWT 作为身份主键；
- 不因用户名修改强制所有设备退出登录；
- 不把用户管理改名接口复用为 server-mode 设密接口；
- 不在运行设置中新增第二个用户名修改接口。
