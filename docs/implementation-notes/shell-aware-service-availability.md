# Shell 感知的服务可用性与本地 core 故障恢复（2026-07-27 设计定稿）

本文记录前端服务可用性机制在平台 Shell 场景下的分层设计决策；
**交付顺序 1–4 已全部实施**（前端纯逻辑见 `frontend/src/service/availabilityPolicy.ts` 及其单测，
Android 自动重启见 `LocalCoreRuntimeManager`），真机验收待办。
涉及 `frontend/src/service/availability.ts`、shell runtime 装配层与各平台 Shell 的服务生命周期职责，
不改动 Shell Bridge v1 契约字段。契约权威见 [`../shell-bridge.md`](../shell-bridge.md)。

## 背景与问题

当前可用性机制是纯 web 语义的单一实现（`frontend/src/service/availability.ts`）：

- 对 `/api/health` 定时轮询：可用 15s / 不可用 5s，单次超时 4s；
- 任何业务请求确认网络失败即调用 `reportServiceUnavailable()`，**单次失败立即**翻转为
  `unavailable`；
- `App.vue` 据此显示全屏 `ServiceUnavailableScreen`（运行设置路由除外），文案语义面向
  "远端服务器连不上，请检查网络"。

这套语义对纯 web / 远端模式是正确的：前端对远端服务只有 HTTP 一种观察手段，失败就该如实呈现。

但在平台 Shell 的 `self-hosted` 模式下（Android 已落地，桌面 Tauri 壳规划中），本地 core 不是
"远端服务器"，而是 App 自身的组件。core 崩溃时立即弹出面向网络故障的全屏错误，存在三个问题：

1. **反应错位**：本地组件故障的第一反应应该是恢复它，而不是向用户报告"连不上服务器"；
2. **信号浪费**：Shell Bridge 快照已包含权威的服务生命周期信号
   （`service.phase: stopped | starting | running | stopping | failed` + `service.error`，
   经 `onRuntimeStateChanged` 推送），比 HTTP 轮询更快更准，但可用性判定完全没有使用它；
3. **过度敏感**：2026-07-26 真机事件（LCSC handler panic，130ms 内自愈，见
   [`lcsc-bag-scanning.md`](lcsc-bag-scanning.md)）证明瞬时故障存在；单次请求失败立即全屏
   会把这类毛刺放大成阻断性错误。

另外，Android 前台以 15s 间隔持续轮询本机回环地址，在推送信号已存在的前提下是纯粹的电池浪费。

## 适用范围

方案按 `RuntimeSnapshot.service.ownership` 分层，而不是按平台分支：

| ownership | 场景 | 可用性语义 |
| --- | --- | --- |
| `local` | Android / 桌面壳 `self-hosted` | phase 推送为权威 + HTTP 低频看门狗 + 自动恢复 |
| `remote` | 纯 web、任何壳的 `connect-to-remote` | 维持现行 HTTP 轮询语义 + 新增去抖 |

桌面 Tauri 壳（`desktop/` 当前为占位实现）实现 Shell Bridge 后自动获得同一行为，无需前端再改。

## 设计决策

### 1. 信号合成：phase 为权威，HTTP 降为看门狗（仅 `ownership = local`）

可用性状态改为二元信号的合成 `f(service.phase, HTTP health)`：

| phase | HTTP 看门狗 | UI 状态 |
| --- | --- | --- |
| `running` | 正常 | `available` |
| `running` | 连续失败（见去抖） | `unavailable`（进程活着但 HTTP 卡死，看门狗兜底） |
| `starting` / `stopping` | 不参与 | `checking`（沿用现有稳定等待反馈，不弹错误） |
| `failed` | 不参与 | 进入自动恢复流程（见决策 2） |
| `stopped`（用户主动） | 不参与 | 明确"服务已停止"状态，**不**自动重启 |

要点：

- HTTP 看门狗**不能删除**：真机 panic 事件证明"进程存活 ≠ HTTP 健康"。但频率从 15s 放宽到
  60s，且窗口不可见时暂停（现有 visibility 逻辑保留）、重新可见时立即补查一次；
- `stopped` 与 `failed` 的区分是 Shell 的责任：由 `stopLocalService` 产生的停止是预期
  `stopped`；崩溃、异常退出、启动失败必须映射为 `failed`。Android `LocalCoreRuntimeManager`
  需确认该映射已成立，这是本方案对 Shell 侧的唯一硬性行为要求；
- 前端装配点在 shell runtime 层（`frontend/src/shell/runtime.ts` 已订阅快照并在
  runtime 变化时调用 `resetServiceAvailabilityForRuntimeChange`），`availability.ts` 增加
  接收外部权威信号的入口，但仍拥有状态机本身。

### 2. 故障的第一反应是自动恢复，不是错误屏（仅 `ownership = local`）

phase 进入 `failed` 时：

- 前端进入"本地服务恢复中"状态：轻量、非阻断的提示（复用 `checking` 语义与现有
  200ms/350ms 稳定等待规范，见
  [`../../frontend/docs/async-state-transitions.md`](../../frontend/docs/async-state-transitions.md)），
  已挂载的路由树与未保存上下文保持不动（沿用 `App.vue` 现有的"断连只盖覆盖层，不销毁页面"原则）；
- **重启循环的所有权在 Shell**（Android `LocalCoreRuntimeManager` / 未来桌面壳运行时管理器）：
  自动重启带次数上限与退避（建议 1–2 次、间隔递增），结果通过快照推送回前端；
  前端只展示进度，不拥有重试计数，避免双方各自重试造成叠加；
- 自动重启耗尽仍 `failed` 时才升级为阻断性错误屏，动作为"重试启动 / 打开运行设置"，
  并展示 `service.error` 的稳定错误信息；
- 现有 `capabilities.restartLocalService` 已在契约中，前端手动"重试启动"直接复用，无契约变更。

### 3. `reportServiceUnavailable` 去抖（所有模式）

单次业务请求失败不再立即翻转 UI：

- 收到失败报告后先立即触发一次健康检查确认（复用现有 `checkServiceAvailability()` 的并发合并）；
- 确认失败才进入 `unavailable`；`ownership = local` 时还应与 phase 信号对照
  （phase 仍 `running` 且健康检查通过 → 视为瞬时毛刺，不翻转）；
- 代价是错误呈现最多延迟一次健康检查往返（秒级），换取瞬时 panic / 单请求毛刺不再闪全屏。

### 4. 错误文案按 ownership 区分语义

- `remote`：维持"服务不可用，请检查网络连接 / 服务器状态"语义；
- `local`：阻断屏必须说"本地服务异常，自动恢复失败"，不得出现"检查网络连接"——
  本地回环没有网络参与，现文案在该场景下是误导。
- `ServiceUnavailableScreen` 按 ownership 切换文案与动作区，不拆分为两个组件。

## 明确不做

- 不改 Shell Bridge v1 契约：`phase`、`error`、`onRuntimeStateChanged`、`restartLocalService`
  与 capabilities 门控全部够用；
- 前端不拥有自动重启循环（只展示 + 手动重试入口）；
- 不为视觉稳定性人为延迟真实请求（延续 `availability.ts` 现有原则）；去抖只推迟**呈现**，
  不推迟探测；
- `client-only` / `server-mode` 的可用性语义不在本文范围（前者无服务，后者壳即服务端）。

## 交付顺序（已全部实施，2026-07-27）

1. `availability.ts` 去抖（决策 3 的 remote 部分）——纯前端、所有模式受益、风险最小；
2. 信号合成与 ownership 分层（决策 1）+ 文案区分（决策 4）——前端 + shell runtime 装配层；
3. Shell 侧 `failed`/`stopped` 映射确认与自动重启循环（决策 2）——这是对**每个拥有本地
   服务生命周期的 Shell** 的统一行为要求，前端只消费 phase 快照、不感知具体平台；
   当前唯一实现是 Android `LocalCoreRuntimeManager`（需真机验收），桌面壳落地
   Shell Bridge 时必须按同样语义实现自己的重启循环；
4. 轮询放宽到 60s 看门狗——依赖 1–3 就位后再改，避免在权威信号接入前削弱现有探测。

实施落点与参数：

- 纯决策规则（信号合成表、轮询间隔、确认复查资格）在 `frontend/src/service/availabilityPolicy.ts`，
  由 `frontend/tests/serviceAvailabilityPolicy.test.mjs`（`pnpm run test:availability-policy`）覆盖；
  状态机与调度仍归 `availability.ts`。新增状态 `recovering`（恢复中轻量提示）与
  `stopped`（用户主动停止的中性阻断屏）。
- 参数：local 看门狗 60s、remote 轮询 15s/5s 不变、确认复查间隔 1s、
  前端恢复窗口 20s（`LOCAL_RECOVERY_WINDOW_MS`）。
- Shell 快照信号在 `shell/runtime.ts` 的 `applySnapshot` 中逐次注入
  （`applyShellServiceStateSignal`）；`startLocalService`/`restartLocalService`
  包装同文件，capability 门控。
- 覆盖层 `ServiceUnavailableScreen` 按 `variant`（remote / local-failed / local-stopped）
  切换文案与主操作；恢复中提示为 `App.vue` 顶部胶囊，复用 200ms/350ms 稳定等待。
- Shell 自动重启（当前实现：Android `LocalCoreRuntimeManager`）：观察到"此前 running 的
  本地服务变为 failed"或恢复链上的再次失败时，按退避 `1s/3s` 最多自动重启 2 次；
  手动 start/stop/restart/apply 会作废挂起尝试并复位计数；`refreshNativeStateIfNeeded`
  把 native 报告的意外非 running 一律映射为 `failed` + `service_crashed`，
  保证用户停止仍是唯一的 `stopped` 来源。未来桌面壳的运行时管理器须复用同一组语义
  （退避上限、手动操作作废、崩溃→failed 映射），参数可按平台调整但语义不得偏离。

## 验收要点

- 真机杀死/模拟 core 崩溃：应看到轻量"恢复中"提示 → 自动恢复成功无阻断；
  重试耗尽 → 本地语义错误屏；
- 运行设置页主动停止服务：显示"已停止"，不触发自动重启，不弹错误屏；
- 瞬时单请求失败（可用 CDP 注入或复现 panic 类毛刺）：不出现全屏闪断；
- `connect-to-remote` 断网/断服务器：行为与现状一致（仅多一次确认往返的延迟）；
- 远端模式与本地模式切换（`resetServiceAvailabilityForRuntimeChange` 路径）：
  状态机正确复位，无跨 runtime 残留。
