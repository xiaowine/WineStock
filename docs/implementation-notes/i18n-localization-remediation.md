# 多语言（i18n）整改方案

状态：实施完成（全量验证通过；Android 设备验证按约定未执行）
适用范围：`frontend`、`core`、`shared`、`server`、`desktop`、`android` 全组件
目标语言：`zh-CN`（默认）+ `en-US`，架构允许后续扩展

## 实施状态（2026-08-10 完成）

- 阶段 0 前端基础设施：✅ vue-i18n 11.4.8、`src/i18n/` 全套（21 域 × 2 语言）、语言切换、路由标题键化。
- 阶段 1 code 化：✅ shared `ConfigValidationIssue.code` + `garde_code` 映射与单测；core 字段校验 `code` + 错误码注册表与测试；前端错误构造期本地化；Shell Bridge `fieldErrors` 三端 `{code,message}` 对齐。
- 阶段 2 文案迁移：✅ 14 个域代理完成约 193 个源文件迁移（详见 `git diff`），zh/en 语言包键完全对齐（`satisfies` 类型门禁）。
- 阶段 3 平台壳：✅ desktop `native_i18n.rs`（sys-locale 0.3.2 + 托盘/门禁 zh/en 表）；android `values-en/strings.xml`（按约定只改未做设备验证）。
- 阶段 4 收尾：✅ `tests/i18nScan.test.mjs` 门禁（硬编码中文扫描 + core/shared 错误码覆盖）；文档（`frontend/docs/i18n.md`、`docs/shell-bridge.md` 决策记录、代码地图）已更新。
- 验证：`pnpm build`（vue-tsc + vite）✅；前端 node 测试 86 通过 0 失败（3 轮稳定）✅；Rust workspace 216 测试全过 + `check --all-targets` 无警告 ✅；i18n 运行时冒烟（语言决议/键对齐/翻译取样）✅；**desktop WebView2 CDP 真机 UI 冒烟**（`pnpm desktop:dev` + 127.0.0.1:9222）✅——zh 默认渲染、偏好设置语言切换、zh↔en 全站文案/`document.title`/`lang` 即时切换、localStorage 持久化、用户/看板/物品页双语言渲染、移动视口 390px 无横向溢出、控制台无 error/warning；冒烟中发现并修复 3 处真问题（导航裸键渲染、物品筛选中文括号残留、纯逻辑模块 i18n 导入约束）。`pnpm format:check` 因本机 git autocrlf=true 导致全仓 CRLF 与 prettier `endOfLine: lf` 冲突而失败（未改动的 `vite.config.ts` 同样失败，属既有环境问题，未做全仓 EOL 重写以避免无关 diff）。Android 设备验证按约定未执行。

## 决策记录（已确认）

- **后端不引入多语言**：`message` 保持中文默认，不引入 Rust 端 i18n 运行时（§4.2）。
- **core/shared 改动范围（选项 A）**：只增加稳定 `code` 字段，不引入后端翻译（§4.2/§4.3）；garde→code 映射在 `shared` 单点实现、core 复用；另加 core 错误码注册表与收集测试作为前端 key 全覆盖的机械依据。
- **前端引入 vue-i18n**：v11.x，Composition API + 类型化语言包（§4.1）。
- **桌面原生 UI 采用方案 A**：`sys-locale` 探测系统语言 + Rust 静态 zh/en 文案表（§4.5）。
- **Shell Bridge 字段错误结构变更**：三端同一改动同步落地，不 bump 协议版本（§4.4）。
- **en 文案产出**：由实施方产出，术语表随实施同步建立（统一中英对照后再批量迁移）；品牌词 WineStock、专有名词（立创/LCSC、WebView 等）保持原文不译（§4.1）。
- **防回退扫描**：作为专门测试项目加入——`frontend/tests/i18nScan.test.mjs` + `pnpm test:i18n-scan`（纳入现有 node --test 体系），阶段 2 迁移完成后启用为门禁（§4.1）。

## 1. 现状盘点

### 1.1 问题定义

用户可见文案分散硬编码在各层源码中：

| 层 | 现状 | 用户可见性 | 主要问题 |
|---|---|---|---|
| `frontend` | 约 223 个 `.vue`/`.ts` 文件直接书写中文文案，模板、脚本、Notice、路由标题、验证提示全部硬编码 | 高（全部 UI） | 无 i18n 库、无资源目录；文案与代码耦合，无法翻译、无法统一管理 |
| `core` | 统一错误响应 `{code, message}` 已稳定（`code` 英文 snake_case，`message` 中文）；字段校验 `details.fields[]` 只有 `{path, message}`，无稳定 code | 中（错误提示、字段校验消息经前端展示） | `message` 被前端直接展示；字段校验消息是 garde 英文模板（如 "length is lower than 8"）与自定义英文码（如 `invalid_json`）混杂，前端无法按码本地化 |
| `shared` | `ConfigValidationIssue { path, message }`，无 code；自定义校验消息即英文码，内置 garde 规则是英文模板 | 低（经 Shell Bridge 字段错误到前端） | 与 core 同类问题 |
| Shell Bridge | `docs/shell-bridge.md` 已规定字段错误应为 `{field, code, message}`，但三端实现仍是 `string[]` | 中（运行设置页字段错误） | 实现落后于已写好的规范 |
| `desktop` | 托盘菜单（"打开/退出 WineStock"）、MessageDialog（WebView2 门禁 4 类失败、前端加载超时）硬编码中文；`ShellRuntimeError.message` 中文；`eprintln` 日志中文 | 高（原生弹窗、托盘）；中（桥错误经前端展示）；低（日志） | 原生 UI 无本地化 |
| `android` | `strings.xml` 已集中 WebView 兼容页文案（基础设施正确）；Kotlin 源码硬编码大量中文 `message`（`NativeContract`、`LocalCoreRuntimeManager`、`AppUpdateManager`、`RemoteRuntimeConfigFallbackValidator`） | 中（桥错误经前端展示、兼容页原生展示） | `strings.xml` 缺语言资源目录；Kotlin 硬编码消息缺 code 化 |
| `server` | 启动摘要、更新检查、错误链全部中文输出到控制台 | 低（运维） | 无 UI 用户，本期不翻译（见 §2 边界） |

### 1.2 已有的正确基础（沿用，不推翻）

- core 错误契约 `{code, message}` 设计正确：`error_response.rs` 注释明确"前端应优先使用 `code` 做分支和本地化"。
- 前端已存在按 code 映射文案的先例：`updates/messages.ts`（与 Android `AppUpdateManager` 的稳定码完全对齐）、`useLcscLookupRequest.ts`、`StockApprovalWorkspace.vue`、`EventsPage.vue`。
- `android/res/values/strings.xml` 已是 Android 标准字符串资源。
- 前端已有版本化 localStorage 偏好模式（`theme/`、`telemetry/consent.ts`、`contact/contactPreferences.ts`），语言偏好沿用该模式。
- Shell Bridge 规范 `docs/shell-bridge.md` 已写明字段错误契约（实现需对齐）。

### 1.3 关键技术事实

- garde 0.23（`Cargo.lock` 确认）的 `Error` 只有 `message` 字段，没有独立 code；`Error::new("code")` 传入的字符串就是 message。内置规则（`length`/`ip`/`range`/`bytes`）产出英文模板消息。
- 前端 `frontend/src/api/errors.ts` 的 `collectFieldErrors` 与 `useFormValidation` 直接把 `{path, message}` 显示到字段；多页面兜底 `error.message`（后端中文）直接上屏。
- vue-i18n 11.x 是 Vue 3 当前稳定版本（Intlify 官方），支持 Composition API、typed message schema、fallback locale。

## 2. 目标与边界

### 目标

1. 所有用户可见文案收敛到各层语言资源，代码中不再出现直接展示用的硬编码文案。
2. 跨层只交换稳定 `code`，文案渲染收敛到前端（UI 层）；后端/壳层 `message` 仅作安全默认与日志兜底。
3. v1 落地 `zh-CN` + `en-US`，语言选择即时生效并持久化。
4. 全离线可用：语言资源随前端打包进 WebView 本地资源，不引入运行时向服务器请求文案。

### 明确不做（本期边界）

- **运维日志不翻译**：`server` 控制台、`desktop`/`android` `eprintln`/`Log` 输出保持中文。日志面向运维与排障，翻译会破坏日志检索一致性；如需可按部署语言另立课题。
- **业务数据不翻译**：用户名、分类名、模板名、库位名、备注等用户数据原样存储展示。
- **第三方内容不翻译**：立创商品描述等外部数据原样展示。
- **服务端不引入 i18n 运行时**：core/shared 不加载语言资源、不做语言协商；语言完全由前端决定。
- **不引入 RTL**：`en-US` 无 RTL 需求；`<html dir>` 由语言包元数据驱动，为未来预留，不做阿拉伯语支持。
- **不做语言懒加载分包**：zh/en 全量打包（WebView 本地资源，体积可控，简单可靠）。

## 3. 核心原则

1. **稳定 code 是跨层契约**：错误、字段校验、桥错误全部以英文 snake_case 稳定码标识；码不随语言变化，文案只随码映射。
2. **文案单点归属**：UI 文案只属于 `frontend`（`i18n` 资源）与平台原生 UI（desktop 托盘/弹窗、android `strings.xml`）；core/shared/壳层不持有展示文案库。
3. **后端 message 保留中文默认**：作为安全兜底与日志可读性；前端命中 code 映射时用本地化文案，未命中时 fallback 到 `message`。
4. **向后兼容**：所有契约变更只增字段、不改既有字段语义；前端按新结构解析，旧结构字段仍存在。
5. **无意义兼容层不保留**：Shell Bridge 结构变更在三端同一改动内完成，不留双格式解析。

## 4. 分组件方案

### 4.1 `frontend`（主战场）

#### 基础设施

- 引入 `vue-i18n`（当前稳定 v11.x，实施时按 `docs/agent-checklist.md` 依赖检查核对版本并记录）。`legacy: false`（Composition API），`fallbackLocale: 'zh-CN'`。
- 新增 `frontend/src/i18n/` 模块域：
  - `index.ts`：`createI18n`、语言决议、持久化、`t` 类型。
  - `locales/zh-CN.ts`、`locales/en-US.ts`：类型化消息目录，键按域组织：`common.*`、`nav.*`、`page.<routeName>.*`、`error.<code>`、`validation.<code>`、`bridge.*`、`update.*`、`notice.*` 等。
  - 语言包用 TS 模块并做类型约束（一份为基准类型，另一份 `satisfies` 同类型），缺 key 在类型检查期报错，保证 223 文件迁移不产生静默缺漏。
- 语言决议顺序：用户持久化选择（`winestock.i18n.locale.v1`，版本化 localStorage，仿 `theme/`）→ `navigator.language` 匹配（`zh*` → `zh-CN`，其余 → `en-US`）→ `zh-CN`。
- 初始化时机：在 `main.ts` 的 `bootstrap` 阶段、主题初始化之后、任何文案渲染之前设置 `locale`；启动漏斗（SetupWizard、RuntimeSettings、ServiceUnavailable）同样走 `t()`。
- 切换入口：`AppPreferencesDialog.vue`（既有桌面/移动通用偏好浮层）新增语言选择；切换即时生效（`locale` 为响应式），持久化。
- `document.title` 同步：路由 `meta.title` 改为消息 key，守卫中按当前 locale 渲染。

#### 错误与校验链路（与 §4.2/§4.4 契约配合）

- `api/errors.ts` 新增统一映射 helper：`apiErrorMessage(error, t)` —— `ApiError` 按 `code` 查 `t('error.' + code)`，key 不存在时返回 `error.message`；网络/响应格式错误等本地类型直接 `t`。
- 字段校验：core 返回 `code` 后，`useFormValidation` 与页面按 `t('validation.' + code)` 渲染，`message` 仅兜底。
- 所有现存 `error.message` 直出点（约 38 个文件命中，如 `StockDraftWorkspace.vue`、`ItemEditorDialog.vue`、`DashboardPage.vue`）替换为上述 helper。

#### 文案迁移方式

- 分域批量替换，每域完成即构建 + 视口回归：
  1. `common`/`nav`/`layout`（AppShell、路由目录标题）；
  2. 认证与启动漏斗（AuthEntry/Login/Register/ChangePassword/SetupWizard/RuntimeSettings/ServiceUnavailable/AppUpdateDialog）；
  3. 业务页面域（看板、物品、出入库草稿与单据、审批、模板、库位、替代料、用户、日志、Events）；
  4. 通用组件与工具层（Notice、Modal、表单、剪贴板、扫码、捐赠、立创、ERP 导入）。
- 模板插值一律走 `t(key, { params })`，禁止 `字符串拼接 ${}`（如 `已移除 ${n} 条`、`无法打开${label}`）。
- 数量/复数：vue-i18n 复数处理；日期与数字格式沿用现状（后端返回的字符串时间原样展示，不做强制格式化）。
- `updates/messages.ts` 等既有 code 映射改为 `t()` 键映射，键名沿用现有码（`update_manifest_invalid` 等）。

#### 防回退扫描（已确认，专门测试项目）

- 新增 `frontend/tests/i18nScan.test.mjs`（Node test runner，仿 `test:theme` 等既有脚本），package.json 增加 `"test:i18n-scan": "node --test tests/i18nScan.test.mjs"`。
- 扫描范围：`frontend/src/**/*.{vue,ts}`；排除 `src/i18n/locales/**`（语言资源）与 `src/api/generated/**`（生成契约）。
- 规则：剥离注释后，源码中出现中文文案字面量（模板文本节点、字符串字面量、模板绑定）即失败；**注释中的中文必须放行**（仓库规范强制中文注释）。
- 启用时机：阶段 2 迁移完成后立即启用（此刻扫描必须零失败）；迁移期间可用显式忽略清单（未迁移文件白名单），迁移完成清零并删除该清单；后续任何硬编码中文文案回潮都会让 `pnpm test:i18n-scan` 失败。
- 实现细节（实施时定稿）：注释剥离 + 字面量匹配用最小自研解析（避免引入新依赖），`.vue` 模板文本与 `<script>` 分别处理。

### 4.2 `core`

- **保持** 统一错误契约 `{code, message}` 不变；`message` 维持中文默认文案。
- **HTTP 字段校验 details 增加稳定 `code`**：
  - 现状：`details = { kind: "validation", fields: [{ path, message }] }`，`message` 是 garde 模板或自定义码，无 code。
  - 目标：`fields: [{ path, code, message }]`（新增 `code` 字段，向后兼容）。
  - 实现：core 新增 garde 消息 → 稳定码映射（覆盖本项目实际用到的规则子集：`length(utf16/bytes)` 的 min/max、`not_blank`、`ip`、`range`、`json`、`code`、权限码等），未命中统一回退 `invalid_field`。映射表 + 单元测试，garde 升级改模板时由测试兜住。
  - 自定义 validator 已用英文码作为 `garde::Error::new` 消息（如 `invalid_json`、`invalid_code`），直接复用为 `code`。
- OpenAPI `summary`/`description` 保持中文（Debug 开发文档，非用户界面）；`details` 字段在 OpenAPI 中是 `serde_json::Value`，本变更不改变 schema，**不需要** `pnpm gen:api` 重生成前端契约类型。

### 4.3 `shared`

- `ConfigValidationIssue` 增加 `code` 字段；`validation_issues()` 输出 garde 消息 → 稳定码映射（与 core 同源策略，shared 放不下 core 的实现时各自维护小表，键命名统一）。
- 自定义校验（`validate_optional_http_url` 等）的 `message` 即稳定码，直接作为 `code` 输出。

### 4.4 Shell Bridge（desktop/android 对齐已写好的规范）

- `docs/shell-bridge.md` 已规定 `RuntimeConfigFieldError { field, code, message }`，三端实现对齐：
  - `frontend/src/shell/contract.ts`：`fieldErrors` 元素从 `string` 改为 `{ code, message }`，`isFieldErrors` 结构校验同步改。
  - `desktop/src/contract.rs`：`RuntimeConfigValidationResult.fieldErrors` 从 `Vec<String>` 改为 `Vec<{ code, message }>`，`prepare_config` 映射 shared `validation_issues` 的 `code`。
  - `android/app/.../RuntimeConfig.kt`：`RuntimeConfigValidationResult.fieldErrors` 同改；`RemoteRuntimeConfigFallbackValidator` 输出稳定码（复用 `ShellErrorCodes` 风格，如 `invalid_port`、`remote_url_invalid`、`local_component_unavailable`）。
- **协议版本决策**：该结构变更属内部开发期契约，三端同仓库同发布、同一改动落地，不 bump 协议版本；在 `shell-bridge.md` 实施记录中注明决策。若未来出现外部旧客户端再走 `bridge_version_mismatch` 路径。

### 4.5 `desktop`（方案 A 已确认）

已决策：桌面原生 UI 采用 **`sys-locale` 探测系统语言 + Rust 静态 zh/en 文案表**（对比过"前端经 Bridge 上报并热重建托盘菜单""引入 fluent/i18n-embed 完整框架"两条路，均不采用：失败路径前端不可达仍需壳层探测、为 6~7 条字符串上框架违背 §2 边界）。

- **托盘菜单**（`tray.rs`）：启动时用 `sys-locale` crate 读取系统 UI 语言，选择 zh/en 静态资源构建菜单；托盘菜单构建一次，语言切换重启后生效（该限制已接受）。
- **MessageDialog**（`main.rs` WebView2 门禁 4 类失败、`commands.rs` 前端加载超时）：同语言决议，静态 zh/en 资源表；诊断码（`WEBVIEW2_MISSING`、`FRONTEND_LOAD_TIMEOUT`）保持不变。
- `ShellRuntimeError.message`（运行配置、防火墙、本地服务错误）保持中文默认不动，前端按 code 覆盖（现状契约如此）。
- `eprintln` 日志不翻译（§2 边界）。
- 明确不做（v1）：托盘菜单运行时热重建（前端偏好经 Bridge 同步后重建）——等真实需求出现再评估。

### 4.6 `android`

- `strings.xml` 已是标准资源机制：新增 `values-en/strings.xml`（`values/` 保留中文为默认语言），无需改代码，系统按设备语言自动切换。
- Kotlin 硬编码中文 `message`（`NativeContract`、`LocalCoreRuntimeManager`、`AppUpdateManager` 等）：保持"稳定 code + 中文默认 message"不变——这些消息经 Bridge 交给前端，前端已按 code 映射（`updates/messages.ts` 与 `AppUpdateManager` 码表已验证一致）。
- `RemoteRuntimeConfigFallbackValidator` 字段错误走 §4.4 的 code 化。

### 4.7 `server`

- 本期不动（§2 边界：运维日志不翻译）。如需，后续可加 `--lang` 或环境变量选择输出语言，本方案不承诺。

## 5. 契约变更清单

| # | 变更 | 影响面 | 兼容性 |
|---|---|---|---|
| 1 | core HTTP 字段校验 `details.fields[]` 增加 `code` | core `http/validation.rs` + 前端错误解析 | 只加字段，向后兼容 |
| 2 | shared `ConfigValidationIssue` 增加 `code` | shared `config.rs` + desktop/android 校验映射 | 只加字段，向后兼容 |
| 3 | Shell Bridge `fieldErrors` 元素 `string` → `{code, message}` | frontend `shell/contract.ts` + desktop `contract.rs` + android `RuntimeConfig.kt`（三端同一改动） | 内部开发期契约，同步落地，不 bump |
| 4 | 新增前端语言偏好 `winestock.i18n.locale.v1` | frontend（不进 Shell 运行配置，仿 consent） | 全新键，无兼容问题 |

## 6. 实施阶段

### 阶段 0：前端 i18n 基础设施
- 引入 vue-i18n；建 `src/i18n/` 与 zh/en 目录骨架（先放 `common`/`error` 最小集）；语言决议 + 持久化 + 切换入口；路由 title 机制改为 key。
- 验证：`pnpm build` 通过；zh/en 切换即时生效；缺 key 走 fallback 不报错。

### 阶段 1：错误与校验链路 code 化
- shared 加 `code` → desktop/android 校验映射；core 加字段校验 `code`（garde 映射表 + 单测）；前端 `apiErrorMessage` helper 与字段校验 `t()` 接入；Shell Bridge `fieldErrors` 结构变更三端同步。
- 验证：`cargo test -p winestock-shared`；`cargo check -p winestock-core -p winestock-desktop`；前端字段错误与 API 错误在 zh/en 下均显示本地化文案；无 contract 校验报错。

### 阶段 2：前端全域文案迁移
- 按 §4.1 的域顺序批量替换 223 个文件；每域完成即构建 + 桌面（1440×900）、断点（768）、移动（390×844）视口检查 + 控制台无新增 error。
- 验证：zh/en 双语言走通核心业务流（登录 → 看板 → 建单 → 审批 → 用户管理）；Notice/Modal/空态/加载态文案均本地化。

### 阶段 3：平台壳本地化
- desktop：`sys-locale` 引入、托盘与 MessageDialog 资源表。
- android：`values-en/strings.xml`。
- 验证：中文/英文系统环境下桌面托盘与门禁弹窗语言正确；Android 英文设备兼容页显示英文。

### 阶段 4：收尾与回归
- 启用防回退扫描门禁（`pnpm test:i18n-scan`，§4.1），迁移期间忽略清单清零并删除。
- `pnpm build`、`pnpm format:check`、`pnpm test:i18n-scan`；`cargo +stable check --workspace --all-targets`（跨 crate 契约变更，扩大验证范围合理）。
- 更新文档：`docs/code-map/frontend.md` 增加 `src/i18n/` 模块域条目；`docs/shell-bridge.md` 记录字段错误结构变更决策；`frontend/docs/` 增加 i18n 使用规范（键组织、错误码映射、禁止硬编码文案）。

## 7. 风险与决策点

1. **garde 消息 → code 映射对 garde 版本升级敏感**：映射表集中 + 单测锁定规则子集 + `invalid_field` 兜底；升级 garde 时先跑映射测试。
2. **迁移量大（223 文件）**：分域推进、类型化语言包把缺 key 前置到编译期；每域独立验证，避免一次大爆炸式替换。
3. **后端 message 语言选择**：保留中文。副作用是 en 用户遇未映射 code 时看到中文兜底——由"前端 code 全覆盖 + 映射表测试"控制，缺 key 归为缺陷。
4. **Shell Bridge 不 bump 版本**：同仓同发布成立；对外发布前如存在旧客户端，改走 `bridge_version_mismatch`（文档记录）。
5. **语言切换时已入队的 Notice 文案**：旧通知保持原语言，切换后新通知用新语言；可接受，记录为已知行为。
6. **桌面托盘菜单语言重启生效**：原生菜单不支持运行时热切，已接受为已知限制（§4.5 决策 A）。
7. **新依赖引入**：`vue-i18n`（v11.x）、`sys-locale`（0.3.x）实施时按依赖检查清单核对当前稳定版本并更新代码地图依赖方向。

## 8. 验收标准

- 全仓库源码（除 `docs/`、日志输出、业务数据、第三方数据）无直接用户展示的硬编码中文文案；文案全部来自语言资源或稳定码映射。
- zh-CN / en-US 双语言完整可用：核心业务流、启动漏斗、错误与字段校验、原生壳 UI（托盘/门禁/兼容页）均正确本地化。
- 契约变更后 `git diff --exit-code frontend/src/api/generated/` 为空（本方案不改变 OpenAPI schema，验证无意外变更）。
- 未映射 code 不崩溃：显示后端默认 `message`，并计入缺陷跟踪。
- 桌面（1440×900）、断点（768）、移动（390×844）三视口无文案溢出与横向滚动；控制台无新增 error/warning。
