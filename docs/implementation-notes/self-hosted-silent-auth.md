# 本机模式免登录：自动管理员 + 静默本地会话（2026-07-27，同日实施完成）

> 实施状态：core（含 7 个新测试）、契约同步、Android 透传（含单测）、前端接线与 UI 均已完成；
> 桌面/Android **实机验证与存量测试库转换尚未执行**（见文末验证计划 3–5）。
>
> **实施偏差**（对下文方案的修正，以此为准）：
>
> 1. **自动开通改为惰性**：不在 core bootstrap 阶段，而在**首次换取时**（`local-session` 端点内，
>    与首用户注册共用同一把写锁）。原因：bootstrap 即开通会让"浏览器直连空库"（当前桌面用法）
>    的首用户注册向导变成死路——admin 已存在但密码无人知晓。惰性开通下，壳内空库首启仍是
>    "首次换取即免登录"，浏览器空库仍走注册向导，两者互不破坏。
> 2. 换取凭据由 `bootstrap_from_config` 在 `self-hosted` 时生成（`LocalSessionSecret`，Debug 脱敏），
>    每次本地服务启动即新凭据；经 `LocalServiceInfo → NativeServiceState → RuntimeSnapshot.service.localAuthExchangeToken` 透传。
> 3. 新增已鉴权端点 `GET /api/auth/local-session/status` 供设密门查询占位状态。
> 4. **前端失败语义细化**：`404 local_session_unavailable`（非本地模式/存量库未转换/浏览器）按
>    普通匿名回落登录流程；`401` 与网络失败落 `authStatus = "unavailable"`，并入 App.vue 既有的
>    服务不可用覆盖层（local-failed 变体带重试），健康检查恢复或 core 重启带来新凭据时自动重试
>    （复用 main.ts 既有联动，未新增 AuthStatus 枚举值，改以 `localSilentAuthActive` 标记静默模式）。
> 5. **账户区呈现修正**：完全隐藏账户身份（头像/用户名/退出登录）与用户管理导航，但顶栏保留一个
>    中性的"本机"选项入口（弹层只含运行设置/局域网地址）——它是 Android 上运行设置的唯一入口，
>    不能随账户区一起消失。

`self-hosted`（仅本机运行）模式下，用户不应看到注册/登录界面。但实现方式**不是绕过鉴权**，
而是"系统里仍有用户和 token，只是用户看不到"：core 自动开通一个默认管理员，
前端经壳内可信通道静默换取正常会话。鉴权中间件、权限体系、refresh 轮换全部原样工作。

## 决策要点

- **不给鉴权中间件加匿名旁路**。core 是 HTTP 服务，即使只绑 loopback，本机其它进程也能
  访问端口；去掉 token 校验等于把全部 API 裸露给任意本地程序。保留 token 后，
  没有凭据依然打不进来，且避免"每个新接口都要考虑匿名分支"的双路径漂移。
- **仅 `self-hosted` 模式免登录**。`server-mode` 面向多客户端，保持现有注册/登录；
  `client-only` / `connect-to-remote` 连远端，登录语义不变。
- **免登录只在存在可信壳通道时生效**。桌面壳方向已确认为 Tauri（与 Android shell
  同构：可信桥 + 内嵌 core），落地时按本方案同机制接入；当前为 Android WebMessage 桥。
  纯浏览器访问（web 平台桥）拿不到换取凭据，回落到现有登录页——这是特性而非缺陷：
  浏览器场景无法区分"本机用户"和"任意本地进程"。
- **业务数据保持可追溯**。单据 `created_by`、审计事件照常记录默认管理员，
  日后切到 `server-mode` 多用户时历史数据语义完整，无需补建用户体系。
- **切 `server-mode` 前必须设真实密码**。默认管理员创建时密码为随机占位（无人知晓），
  本机模式无感；一旦要对局域网开放，先强制设置一次真实密码，这是免登录与共享之间的门。

## 方案总览

三个新机制，均为小改动面：

1. **自动开通默认管理员**（core bootstrap）：`self-hosted` 且库中无任何用户时，
   自动创建用户 `admin`（用户名经用户确认；随机占位密码 + 全部内置权限，复用现有
   "首个注册用户自动获全权限"的逻辑），并在数据库托管鉴权设置中记录两行：
   `local_auto_login_user_id`（静默会话的目标用户）与
   `local_auto_login_password_placeholder`（密码仍为占位标记）。
   用设置行而非用户表加列，**不改 schema、不需要存量库 ALTER**。
2. **每次启动的换取凭据**（exchange secret）：`start_local_service` 在 `self-hosted`
   模式下生成一次性的进程内随机凭据（`random_urlsafe(32)`），放入 AppState 并随
   `LocalServiceInfo` **进程内**交给平台壳——不经未鉴权 HTTP 暴露、不写日志。
   壳通过既有可信桥（Android 仅信任 `https://winestock.internal` origin 的
   WebMessage 通道）随 RuntimeSnapshot 交给前端。
3. **静默换取端点** `POST /api/auth/local-session`：匿名路由，但仅当 AppState 中存在
   换取凭据（即 `self-hosted` 模式）才可用，常数时间比较凭据后按
   `local_auto_login_user_id` 签发与 `POST /api/auth/login` 完全相同的
   `AuthTokenResponse`（access + refresh）。其余模式下该端点直接 404/403。

前端在 `ownership === "local"` 且快照携带换取凭据、当前会话为匿名时自动调用换取并
`establishAuthSession()`，登录页整个流程被跳过。**本地静默模式下不存在"回落登录页"**：
自动开通的管理员密码是随机占位、无人可输，登录页是死胡同。换取失败一律由前端控制层
按服务可用性语义处理——面向用户给出错误提示并自动重试，登录/注册路由完全不参与。

## core 改动

| 位置                                                          | 内容                                                                                                                                                                                   |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `local_service.rs` + `state.rs`                               | `self-hosted` 时生成 per-boot 换取凭据入 AppState；`LocalServiceInfo` 增字段（仅进程内传递）                                                                                           |
| `users/service/register.rs` 或新 `users/service/bootstrap.rs` | 抽出"创建用户 + 授予全部内置权限"共用逻辑；bootstrap 阶段 `self-hosted && !has_users` 时创建 `admin`（随机占位密码），写入两行鉴权设置                                                 |
| `auth/bootstrap.rs`                                           | 自动开通后 `admin_setup_required` 为 false（`has_users` 已真），无需额外改动，确认即可                                                                                                 |
| 换取目标自愈（users bootstrap 同处实施）                      | `self-hosted` 启动校验 `local_auto_login_user_id`：缺失/未激活/权限不全时自动修复并写 `audit_events`（见"模式切换语义"）                                                               |
| `auth/mod.rs` + `auth/controller.rs` + `auth/service.rs`      | 新增 `POST /api/auth/local-session`：校验换取凭据（常数时间比较）→ 复用 login 的 token 签发路径（抽共用函数），`device_name` 固定如 `local-shell`；非 `self-hosted` 模式不注册该路由   |
| `auth`（改密）                                                | 改自身密码接口：当 `local_auto_login_password_placeholder` 标记当前用户时，允许**不提供旧密码**直接设置新密码，成功后清除占位标记（占位密码无人知晓，否则永远设不了真实密码）          |
| `users/service/management.rs`                                 | 管理员重置该用户密码同样清除占位标记（保持一致）                                                                                                                                       |
| `tests/`                                                      | 新增：`self-hosted` 空库自动开通；换取成功/凭据错误/非本地模式 404；占位态免旧密码改密并清标记；`server-mode` 空库仍走首用户注册；标记用户被停用/收权后 `self-hosted` 启动自愈并留审计 |

## 契约同步

core 改完执行 `cd frontend && pnpm gen:api`，前端获得 `local-session` 端点类型。

## Android 壳改动

| 位置                                                       | 内容                                                                                                                              |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `android/native/src/contract.rs` + `engine.rs`             | `NativeServiceState` 增可选 `local_auth_exchange_token`，running 态投影时填充                                                     |
| `NativeContract.kt` / `NativeCoreClient.kt`                | JSON 解析增字段                                                                                                                   |
| `RuntimeSnapshotFactory.kt` + `LocalCoreRuntimeManager.kt` | `RuntimeServiceSnapshot` 增字段并透传进快照 JSON（注意现状：`admin_setup_required` 在 Kotlin 层被丢弃未透传，本字段不要重蹈覆辙） |

## frontend 改动

| 位置                        | 内容                                                                                                                                                                                                                                               |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shell/contract.ts`         | `RuntimeSnapshot.service` 增可选 `localAuthExchangeToken`（仅 local + running 快照携带）；web 桥永远不提供                                                                                                                                         |
| `shell/runtime.ts`          | `applySnapshot` 把凭据交给 auth 层（快照变更/服务重启时更新）                                                                                                                                                                                      |
| `auth/session.ts`           | `ensureAuthSessionInitialized` 流程内：匿名且有换取凭据 → 调 `local-session` 换取并 `establishAuthSession`；refresh 失败（本地库 7 天 refresh 过期等）时同样自动重新换取。新增本地会话状态（如 `local-auth-failed`）供控制层消费，不落入普通匿名态 |
| 路由守卫 `router/guards.ts` | 本地静默模式（`ownership === "local"` 且快照应携带换取凭据）下，匿名/会话失败**不重定向 auth-entry/login**；换取失败呈现为错误提示态，路由停留原地。远端模式守卫行为不变                                                                           |
| 控制层错误呈现              | 换取失败并入既有 Shell 感知可用性分层（`service/availability.ts` / `availabilityPolicy.ts` 的错误呈现路径）：显示"本地会话建立失败"类提示 + 重试按钮；core 重启产生新快照/新凭据时自动重试恢复                                                     |
| UI                          | 本地静默模式下**完全隐藏账户区**（含头像/当前用户/退出登录/改密/用户管理入口，经用户确认）——界面彻底无账号感；登录/注册页与账户区仅服务于远端模式与纯浏览器访问。切 `server-mode` 的强制设密引导对话框是本地模式下唯一的密码设置入口               |
| `RuntimeSettingsPage.vue`   | 切换到 `server-mode` 前检查占位标记（经换取会话调用状态接口或复用 `/api/auth/me` 扩展字段）：仍为占位 → 弹出"设置管理员密码"流程（走免旧密码改密），完成后才允许提交模式切换                                                                       |

## 存量库处理

现有桌面/Android 测试库已各有真实用户，不满足"空库自动开通"条件。按项目惯例原地转换：
向鉴权设置表插入 `local_auto_login_user_id = <现有管理员 id>`（**不**插占位标记，
密码是真实的），即可让存量库同样享受静默登录，且原密码继续可用。
不做自动识别（"库里恰好只有一个用户就自动选中"之类的启发式有误绑风险）。

## 模式切换语义

免登录只是 `self-hosted` 的会话建立方式，数据库与用户体系在所有模式间共享、切换不触碰数据。
各方向切换后的行为：

| 切换                                                | 行为                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `self-hosted` → `server-mode`                       | 前置门：占位标记仍在则先强制设真实密码（见 frontend 改动）。切换后换取端点关闭，宿主 UI 与其它客户端一样走登录页；`admin` 及其历史单据（`created_by` 等）原样保留，用刚设的密码登录即可。此前静默会话持有的 refresh token 若 `api_base_url` 未变仍可续期（本人合法凭据，无需强制吊销）；地址变化则自然失效落到登录页 |
| `server-mode` → `self-hosted`                       | 换取端点重新启用，静默会话自动恢复，无需任何操作；期间设置的真实密码保留且继续可用（静默与密码登录不互斥）                                                                                                                                                                                                           |
| `self-hosted` → `client-only` / `connect-to-remote` | 本地 core 停止，前端连远端，走远端服务器的正常登录。本地 refresh token 因绑定 `api_base_url` 不会误用于远端；本地库、`local-admin`、鉴权设置行全部原样保留                                                                                                                                                           |
| `client-only` / `connect-to-remote` → `self-hosted` | 壳启动本地 core：本地库为空则触发自动开通（首次即免登录）；已有标记则直接静默恢复。之前远端会话的 refresh token 留在 localStorage（按远端 URL 隔离），下次切回远端仍可续期                                                                                                                                           |

**换取目标自愈**（多用户库切回本机的边界）：`server-mode` 期间 `admin` 可能被
停用或收权。`self-hosted` 启动时 core 校验 `local_auto_login_user_id`：用户缺失、
未激活或权限不全时自动修复（重新激活 / 补齐全部内置权限 / 极端情况下重建标记用户），
并写入 `audit_events` 留痕。依据：本机模式下设备物理持有者即最高权限——SQLite 文件
本就在其手中，自愈只是把这一事实变成可用的 UI，而不自愈会导致本机界面永久错误态
（本地模式无登录页可逃生）。

## 异常与降级语义

- 换取失败（凭据过期快照未更新、core 刚重启等）：**不进登录页**，前端控制层按可用性
  错误语义面向用户提示并可重试；本地 core 重启后新快照携带新凭据，自动重试恢复，
  与现有 Shell 感知可用性机制（phase 推送权威 + 看门狗）同一套驱动。
- 用户强行在占位密码状态下配置成 `server-mode`（绕过前端门）：换取端点在该模式不可用、
  占位密码无人知晓 → 无人能登录，但**不会砖**——切回 `self-hosted` 静默会话即恢复，
  再按引导设密码。core 在此状态启动时打一条警告日志即可，不做硬阻止。

## 安全边界（记录为不变量）

- 换取凭据：每次进程启动重新生成、只存内存、只经进程内 `LocalServiceInfo` → 壳桥传递、
  不落盘不入日志、常数时间比较。
- HTTP 端口上的所有业务路由鉴权行为与现在完全一致；本方案未新增任何匿名业务面。
- 唯一新增匿名端点 `local-session` 仅 `self-hosted` 注册，且无有效凭据时不泄露任何信息。

## 验证计划

1. `cargo test -p winestock-core`（上表新增用例）；
2. `pnpm gen:api` 后 `vue-tsc` 与前端测试；
3. Android 实测：空库首启无登录页直达主界面；杀 core 自动重启后会话存活；
   人为制造换取失败（如篡改凭据）验证出现的是错误提示态而非登录页，重启 core 后自动恢复；
   切 `connect-to-remote` 出现登录页、切回 `self-hosted` 恢复静默；
   切 `server-mode` 被引导设密码；
4. 存量测试库插设置行后验证静默登录且原密码仍可用；
5. 浏览器直连（web 桥）验证仍走登录页。

## 明确不做

- `server-mode` 宿主机免登录（多客户端语义下宿主也正常登录）；
- 桌面免登录本次不做：桌面壳方向已确认为 Tauri（与 Android shell 同构），落地时按本方案
  同机制接入换取凭据即可；过渡期 server 二进制 + 浏览器直连维持登录页（用户确认可接受，
  refresh token 7 天续期下实际登录频率很低），不做 token-URL 之类的浏览器免登录变通；
- 本地静默模式下回落登录页（占位密码无人可输，登录页是死胡同；一律走可用性错误提示 + 自动重试，登录/注册路由只服务远端与纯浏览器场景）；
- 免登录开关 UI（`self-hosted` + 可信壳即默认启用；存量库经设置行转换）；
- 基于 loopback 对端地址的免凭据放行（无法区分本机其它进程，明确否决）。
