# 首次初始化向导（2026-07-27 设计定稿，迁移自"首启强制进运行设置页"）

本文记录首次启动漏斗从运行设置页迁移到初始化向导的设计决策与实施边界。
当前状态：**已实施**（`frontend/src/pages/SetupWizardPage.vue`、`telemetry/consent.ts`、
`runtimeReadiness.shouldEnterSetupWizard` + 守卫分流；浏览器 CDP 已验证两条路径全流程、
固定尺寸、校验拦截与已初始化重定向）。原型为桌面临时文件（不入库），定稿结论收敛于本文。

实施偏差说明：「Android native 不可用时置灰本机选项」未实现——Shell Bridge v1 契约没有
`nativeAvailable` 信号（`capabilities.startLocalService` 在未配置状态恒为 false，不能区分
平台能力与当前状态）。当前行为与运行设置页一致：允许选择，apply 失败时呈现
`native_library_unavailable` 稳定错误并可返回改选远端。若后续要做预置灰，需先给契约补
平台能力信号，属于契约变更，单独评估。
现行漏斗的权威描述见
[`../../frontend/docs/implementation-notes/runtime-first-startup-funnel.md`](../../frontend/docs/implementation-notes/runtime-first-startup-funnel.md)，
本文只描述迁移差异，不重复其内容。

## 背景与目标

现行首启漏斗在 Shell 未初始化时强制进入运行设置页（`guards.ts` 守卫第 2 步）。
运行设置页是面向维护的完整配置界面，作为新用户第一屏存在两个问题：

1. 暴露 bindHost/端口/局域网访问等首次启动不需要的决策，认知负担重；
2. 无法承载"首次体验"应有的欢迎语义与后续扩展（数据收集同意、未来的语言/主题）。

目标：新增专用初始化向导路由接管**首次未配置**状态；运行设置页降级为
登录后的维护入口与异常修复路径，自身不做任何改动。

## 向导定位（不可违背的边界）

- 向导是**薄壳**：不拥有配置逻辑，收集完选择后在最后一步调用一次现有
  `applyRuntimeConfig`，权威校验、self-hosted `port=0` 分配、本地启动全部走既有链路；
- 只服务 `configStatus = "unconfigured"` 且 `initialized = false`；
  `configStatus = "invalid"`（配置损坏）**不进向导**，维持现行运行设置页修复路径；
- 中途退出无副作用：apply 是原子的最后一步，未完成则下次冷启动照常重进向导；
- 完成语义复用现有 `runtimeSetupFinished`（Shell `initialized` + 服务可 HTTP），
  **不新增**"已看过向导"之类的本地标记——避免"标记完成但配置未 apply 成功"的分叉；
- 向导内**无**进入运行设置页的跳转（已确认）。

## 页面结构与文案（定稿）

上限 3 页，每页只回答一个问题；本机路径实际 2 页。所有页共享固定尺寸面板。

| 页 | 标题 | 内容 | 默认值 |
| --- | --- | --- | --- |
| 1 | 欢迎使用 WineStock | 说明"先选择这台设备的使用方式，稍后可以随时在设置中更改"；两张单选卡片 | 仅在本机使用（带「推荐」徽标） |
| 2（条件） | 连接服务器 | 仅选"连接已有服务器"时出现；地址输入 + 测试连接 | — |
| 3 | 帮助改进 WineStock | 匿名使用数据开关 + "了解收集哪些内容"详情入口 | 关（opt-in） |
| 完成态 | — | 中性"加载中…"（不暴露服务/连接等实现细节）→ 就绪后自动进入现行 `/auth` 漏斗 | — |

卡片文案定稿：

- 仅在本机使用（推荐）：数据保存在这台设备上，无需网络即可使用。
- 连接已有服务器：多台设备共享同一台服务器上的数据。
- 数据收集开关说明：帮助开发者定位和排查问题；不包含库存内容与账户信息，仅在联网时生效。
  分析服务由 Microsoft Clarity 提供；说明行附「查看 Microsoft 隐私声明」外链
  （`TELEMETRY_POLICY_URL`，经 `openExternal` 打开，向导与偏好设置 Dialog 共用同一文案与链接）。
  默认关闭，可随时在「偏好设置」中更改（与偏好设置 Dialog 文案一致，说明开启的动机）。

文案红线：全程不得出现 self-hosted、API 根地址、端口、bindHost 等术语；
"推荐"徽标只出现在本机选项上（teal 徽章样式）。

## UI 实现约束（原型 CDP 实测得出，逐条为踩过的坑）

1. **固定尺寸**：所有步骤（含 apply/完成态）共享固定尺寸内容区与固定高度标题区，
   步骤内容不得决定容器尺寸；完成态隐藏页脚用 `visibility: hidden` 保占位，
   禁止 `display: none` 导致按钮行塌陷。
2. **切换动画**：方向性滑入（前进从右、后退从左），参数用现有动效 token
   （160ms / `cubic-bezier(0.2, 0, 0, 1)` / 18px 位移），`prefers-reduced-motion` 下禁用。
3. **选中态与焦点环分离**：卡片选中态 = accent 边框 + `accent-soft` 底色；
   焦点环只服务键盘导航（`:focus-visible` 系，Vue 实现用 `:has(input:focus-visible)`
   或等价类绑定），样式对齐 `_base.scss` 约定——`3px rgb(111 42 54 / 24%)` 光环、
   offset 2px；鼠标点选不得出现焦点环（`:focus-within` 会误伤，禁用）。
4. **不裁切光环**：焦点光环向卡片外扩 5px，步骤容器不得设置 `overflow: hidden`
   （滑动动画由面板 28px 内边距容纳，无需裁切）。
5. 圆角一律使用项目 token（4/6/8px）；徽标用 `--radius-sm`，卡片/面板用 `--radius-lg`。

## 路由与守卫迁移

改动集中在一处分流（`frontend/src/router/guards.ts` 守卫第 2 步）：

```text
现行：!runtimeSetupFinished            → 运行设置页
迁移后：
  configStatus=unconfigured 且未初始化 → /setup（向导，requiresService=false、requiresAuth=false）
  其余未就绪（invalid / 已配置但服务未就绪）→ 运行设置页（现行为不变）
```

- 新路由 `/setup`（`setup-wizard`）为独立响应式页面，与运行设置页同层
  （不在 AppShell 内）；已初始化后访问 `/setup` 重定向回业务默认页；
- 运行设置页职责收敛为：登录后账户菜单入口 + `invalid`/服务故障修复路径；
  页面本身零改动；
- 完成后的去向沿用现行漏斗：apply 成功 → `runtimeSetupFinished` 变 true →
  守卫自然放行到 `/auth`（注册/登录）。若本机免登录方案落地，
  本机路径完成后将直接进入业务页，向导不需要感知该差异。

## 数据收集偏好与 Clarity 边界

- 偏好是**前端自有**持久化（localStorage，键名建议 `winestock.telemetry.consent`，
  值含版本号以便将来文案变更后重新征询），不进 Shell 运行配置、不动 Bridge 契约；
- 同意前一个字节都不加载：Clarity SDK（`@microsoft/clarity`）动态 import，
  仅在 consent=true 且运行环境可出网时初始化；拒绝或未选择时不 import；
  **已接入**（`telemetry/clarity.ts`，项目 xsl053wgz1）：装配时按持久化偏好补启动、
  向导同意后立即启动；Network 面板已验证未同意零请求、同意后 tag 加载；
- 设置页后续补一个可随时开关的入口（不属于本次迁移范围）；
- Android self-hosted 本地回环场景天然不出网，文案"仅在联网时生效"已覆盖该预期。

## 多语言 / 主题预留（原则性结论）

语言跟随系统语言、主题跟随系统深浅色，**都不新增向导页面**——
"一页只放必须由用户决策的问题"。将来多语言落地时在欢迎页角落放当前语言切换器；
主题进设置页。向导总页数上限 4（含未来可能的合并"个性化"页）。

## 平台边界

- Android native 不可用（`nativeAvailable=false`）：第 1 页"仅在本机使用"置灰
  并显示一行原因，仅"连接已有服务器"可选（现行降级语义的向导化呈现）；
- 纯网页端（快照 `platform === "web"`）：只有「连接已有服务器」一种能力，使用方式页
  没有可做的决策，**整页跳过**——向导直接从服务器地址页开始（服务器地址 → 数据收集，
  共 2 页）；远端路径行为与真实 Shell 一致；
- 桌面壳落地 Shell Bridge 后自动获得向导，无需前端再改。

## 迁移交付顺序

1. 向导页面组件 + `/setup` 路由（纯新增，不接守卫，可用 URL 直达验证）；
2. 守卫分流切换（`unconfigured` → 向导）+ 已初始化访问 `/setup` 的重定向；
3. 数据收集偏好模块（持久化 + 设置页开关暂缓）；Clarity 接入已完成
   （`telemetry/clarity.ts` 按需加载）；
4. 文档同步：`runtime-first-startup-funnel.md` 与 `frontend/docs/routes.md`
   补充向导路由与新分流。

## 验收要点

- 首次冷启动（unconfigured）：进向导；本机路径两次"下一步"完成并自动进入注册页；
- 服务器路径：地址校验错误在向导内呈现（复用 shared 校验错误映射），不跳出向导；
- apply 失败（端口异常、native 故障）：停留在向导显示稳定错误，可重试；
- `configStatus=invalid` 冷启动：进运行设置页而非向导（现行为）；
- 已初始化后直达 `/setup`：重定向业务默认页；
- 向导中途杀进程/刷新：下次仍进向导，无半完成状态；
- Android native 不可用：本机选项置灰 + 原因，仅远端可走通；
- 数据收集默认关；同意后偏好持久化，Network 面板确认未同意时无 Clarity 请求
  （已验：未同意零请求，同意后加载 `clarity.ms/tag/<projectId>`）；
- UI 约束逐条复核：面板尺寸全流程恒定（原型实测 440×420 基准）、鼠标无焦点环、
  键盘光环完整不裁切、`prefers-reduced-motion` 无动画。

## 明确不做

- 不改运行设置页、Shell Bridge 契约与 `runtimeReadiness` 语义；
- 不新增"已看过向导"标记；
- 不在向导内提供高级配置（bindHost/端口/server-mode/局域网访问）与设置页跳转；
- 语言/主题不在本次范围（Clarity SDK 接入已于后续任务完成，见上文）。
