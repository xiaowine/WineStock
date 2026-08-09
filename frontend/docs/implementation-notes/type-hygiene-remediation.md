# 前端类型卫生整改方案（any / unknown 盘点与治理）

> **实施状态：已完成（2026-08-10）。** 三阶段全部落地，含连带发现的 AppPreferencesDialog 硬编码文案 i18n 回归修复；实施记录见文末。

## 现状盘点（2026-08-10，`vue-tsc -b` 基线通过）

全仓扫描 `frontend/src`（排除 `api/generated/` 生成物）：

| 类别                                     | 数量              | 说明                                                                                                                                                                          |
| ---------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 显式 `any` 类型                          | **0 处**          | 全仓仅 3 个 "any" 单词：`client.ts` 的 `AbortSignal.any(...)` API 调用 1 处、en-US 语言包英文文案 2 处，均不是类型注解                                                        |
| `unknown`（手写代码）                    | ~195 次 / 71 文件 | 其中约 3 次为英文文案（locale 文件），实际类型用途 ~192 次                                                                                                                    |
| `unknown`（`api/generated/schema.d.ts`） | 306 次            | openapi-typescript 对 core OpenAPI 的忠实翻译（响应头 `[name: string]: unknown`、`ApiErrorResponse.details`、事件 `details`），不可手改，只能由 core 侧收紧 schema 后重新生成 |

**结论：`any` 实质上不存在，"任意类型放弃静态检查"的 bug 源在当前代码中几乎没有。** `unknown` 的大头是**刻意且正确**的信任边界防御，不是隐患；真正可收窄的懒类型点约 8 处，见下。

## 分类

### A. 信任边界 `unknown`（正确模式，必须保留，不是整改对象）

这些位置的数据来源不受 TypeScript 静态类型约束，`unknown` + 运行时守卫是文档认可的既定模式（见代码地图"Shell Bridge 契约与运行时结构校验（不能只信任 TypeScript 静态类型）"）：

- `shell/contract.ts`（16 处）：Shell Bridge 快照的 `assertXxx(value: unknown)` / `isXxx(value: unknown)` 运行时结构校验。
- `shell/transports/*`、`transports/bridgeError.ts`（9 处）：传输边界错误规范化。
- `api/client.ts`（6 处）：`readResponsePayload(): Promise<unknown>`、`parseXhrPayload`、multipart `payload: unknown`、`json?: unknown` 请求体——fetch/XHR 边界。
- `api/errors.ts`（7 处）：`details: unknown`、`isApiErrorResponse(value: unknown)`、`isRecord` 等统一错误契约运行时校验。
- `pages/events/details.ts`（16 处）：后端审计事件 `details` 是任意 JSON（生成 schema 亦为 `unknown`），解析必须守卫。
- `auth/storage.ts`、`donation/model.ts`、`theme/model.ts`、`i18n/model.ts`、`telemetry/consent.ts`、`clipboard/model.ts`、`components/attributes/imageDraft.ts`（`isImageDraftValue`）等：localStorage / 外部输入规范化入口。
- 全部 `catch ((error: unknown) => ...)` 配合 `instanceof` / 类型守卫收窄（约 40+ 页面与组件）：标准错误处理。
- `router/guards.ts` 的 `router.replace(...).catch((error: unknown)`：vue-router 导航失败。
- `directives/copyable.ts`：`Directive<HTMLElement, unknown>`——Vue 指令 binding 值运行时解析，比 Vue 自带的 `any` 更严。
- `EventsPage.vue` / `StockApprovalWorkspace.vue` 的 `Record<string, unknown>` query 解析、`stringQuery`/`dateQuery` 归一化：URL 输入不可信。

这些位置若改回具体类型反而会引入"静态类型骗过运行时"的 bug。**整改边界中明确排除。**

### B. 可收窄的懒类型点（整改对象，约 8 处）

| #   | 位置                                                                                           | 现状                                            | 目标                                                                                                                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `i18n/index.ts:70` `translateTitle(key: unknown)`                                              | 参数放太宽                                      | 先 `lsp references` 查调用方；`router/meta.d.ts` 已增强 `meta.title: string`，`applyDocumentTitle(title: unknown)`（:133）应收窄为 `string`，`translateTitle` 随之收窄为 `string`（保留"非键原样返回"语义） |
| 2   | `router/guards.ts:149` `resolvePostLoginLocation(router, redirect: unknown)`                   | 登录回跳目标                                    | 调用方只传字符串或 null（query 参数/localStorage），收窄为 `string \| null`，内部 `typeof redirect !== "string"` 防御保留或简化                                                                             |
| 3   | `pages/OutboundOrdersPage.vue:471` `function iso(v: unknown)`                                  | 无显式返回类型（同文件 `local`/`toIso` 也未标） | 补 `: string`；核对 InboundOrdersPage 同族函数统一风格                                                                                                                                                      |
| 4   | `updates/messages.ts:4` `updateErrorCode(error: unknown): unknown`                             | 返回 unknown                                    | 收窄为 `string \| undefined`（`code` 字段 typeof 检查）                                                                                                                                                     |
| 5   | `api/health.ts:12` `apiClient.request<unknown>`                                                | 泛型给 unknown                                  | 改为 `request<HealthResponse>`，`isHealthResponse` 运行时守卫保留（守卫不因静态类型而删除）                                                                                                                 |
| 6   | `navigation/nativeBackCore.ts:98` `resolve(resolution): Promise<unknown>`                      | 平台应答结果                                    | 查装配层（`navigation/`）实际返回后收窄，通常为 `Promise<void>`                                                                                                                                             |
| 7   | `components/stock-draft/StockDraftWorkspace.vue:399-404` 插槽 `(): unknown`                    | 6 个插槽声明                                    | 收窄为 Vue 插槽真实返回 `VNode[]`（`vue` 的 `Slot` 类型），或按使用处具体化                                                                                                                                 |
| 8   | `components/forms/SelectControl.vue` `defineModel<unknown>()`、option `value: unknown`（4 处） | 泛型控件全 unknown                              | Vue 3.5 `<script setup generic="T">` 泛型化（见 Phase 2，连锁影响多个调用方的 `change` 回调 `value: unknown`）                                                                                              |

## 整改方案（分三阶段）

### Phase 1：低风险收窄（B 表 #1–#7）

逐个改动，每处先 `lsp references` 确认调用方，改后立即 `npx vue-tsc -b`。
`#7` 涉及插槽渲染类型，需先读 `StockDraftWorkspace.vue` 的 slots 消费处确认实际返回。

### Phase 2：SelectControl 泛型化（#8，已实施）

- `SelectControl.vue` 改为 `<script setup lang="ts" generic="T">`，option `value: T`、`defineModel<T>()`、`change: [value: T]`。
- 连锁收窄调用方：`TemplateEditorDialog.vue:526,544`、`EventsPage.vue:528,538` 等 `change` 回调的 `value: unknown` → 具体联合类型。
- 收益：表单值与列表查询筛选拿到类型收窄；成本：SFC 泛型引入 + 调用点逐个核对，独立提交、独立验收，不阻塞 Phase 1/3。

### Phase 3：防回归门禁与规范

1. **新增 `frontend/tests/typeHygieneScan.test.mjs`**（沿用现有 node --test + TypeScript 转译模式，与 `i18nScan.test.mjs` 一致）：
   - 扫描 `src/**/*.{ts,vue}`（排除 `api/generated/`），正则命中 `: any`、`as any`、`any[]`、`<any>`、`Record<string, any>`、`Promise<any>` 等任何显式 `any` 类型即测试失败；
   - `unknown` 只统计不拦截（合法模式），报告数量变化便于人工关注趋势。
   - `package.json` 增加 `"test:type-hygiene": "node --test tests/typeHygieneScan.test.mjs"`。
2. **新增 `frontend/docs/type-safety.md`** 并登记到 `frontend/docs/README.md` 索引：定义本项目"信任边界必须 `unknown` + 运行时守卫；业务内部流转必须具体类型；禁止显式 `any`"的约定，附 B 表作为反例参考。
3. **（独立决策项，不并入本次）** 引入 ESLint + typescript-eslint（`no-explicit-any`、`no-unsafe-*` 规则族）。项目当前无任何 lint 基建，属于新依赖 + flat config + 全仓适配的工程变更，建议作为独立任务立项；门禁测试可先行覆盖其核心诉求。

## 明确不做（防范围蔓延）

- 不改 `api/generated/schema.d.ts`（生成物，只能由 core 侧 OpenAPI 收紧后 `pnpm gen:api` 重新生成；`ApiErrorResponse.details` 与事件 `details` 按契约设计即为任意 JSON，core 侧也无需改）。
- 不动 A 类信任边界代码——那正是本项目防运行时注入 bug 的防线。
- 不引入 ESLint（见 Phase 3 第 3 条）。

## 验证清单

- 每阶段 `npx vue-tsc -b` 通过（当前基线已验证通过）。
- Phase 3 后 `pnpm test:type-hygiene` 通过；`pnpm format:check` 通过。
- 涉及 UI 的改动（#7、#8）按 `ui-consistency-checklist.md` 在桌面/移动视口冒烟验证。
- 未涉及 core HTTP 契约变更，无需 `pnpm gen:api`。

## 实施记录（2026-08-10）

### 已落地

- **Phase 1（#1–#7 全部收窄）**：`i18n/index.ts`（`translateTitle`/`applyDocumentTitle` → `string`）、
  `router/guards.ts`（`resolvePostLoginLocation` redirect → `LocationQueryValue | LocationQueryValue[]`）、
  `OutboundOrdersPage.vue`（`iso` 补 `: string`）、`updates/messages.ts`（`updateErrorCode(): string | undefined`）、
  `api/health.ts`（`request<HealthResponse>`，守卫保留）、`nativeBackCore.ts`（`resolve(): Promise<NativeBackResolutionAck>`）、
  `StockDraftWorkspace.vue`（6 个插槽 → `VNode[]`）。
- **Phase 2（SelectControl 泛型化）**：`<script setup generic="T">`、`defineModel<T>()`、`change: [value: T]`；
  连锁收窄 `EventsPage.vue` 两个 change 回调 → `string`、`TemplateEditorDialog.vue` 两个回调 → FormSelect 契约联合。
- **Phase 3（门禁与规范）**：新增 `tests/typeHygieneScan.test.mjs`（命中显式 `any` 即失败，`unknown` 只统计趋势）、
  `package.json` 注册 `test:type-hygiene`、新增 `type-safety.md` 并登记 README 索引、`agent-checklist.md` 增加类型卫生约束条目。
- **连带修复（i18n 回归）**：整改中 `i18nScan` 门禁红灯，定位为 UI 修改硬编码 `语言 / Language` 标题与语言自名标签；
  `AppPreferencesDialog.vue` 迁移到既有键（`components.languageHeading`/`components.appLanguage`/`common.languageZh`/`common.languageEn`）；
  `i18n.md` 规则 1 强化为"禁止硬编码任何语言的 UI 文本"并注明扫描只拦中文；`agent-checklist.md` 增加 i18n 文案约束条目。

### 与方案的实际偏差

- B 表 #2：`redirect` 最终收窄为 `LocationQueryValue | LocationQueryValue[]`（vue-router `route.query` 的实际类型），而非方案初稿的 `string | null`。
- B 表 #8：泛型首版带 `T extends string | number | boolean | null | undefined` 约束，被 `ItemAttributeEditor` 的
  `attribute.value` 联合类型（含 `ImageDraftValue`）击穿，改为无约束 `generic="T"`，各调用点仍按 v-model 推断具体类型。
- Phase 2 方案初稿标注"可选"，经确认后一并实施。

### 验证结果

- `npx vue-tsc -b`：通过。
- `pnpm test:type-hygiene`：2/2 通过，显式 `any` 0 处，`unknown` 词频趋势 167 次 / 228 文件（不含生成物与语言包）。
- 全量 `node --test tests/*.test.mjs`：89 tests / 0 fail（含 i18nScan 从红转绿）。

### 遗留事项（未处理，独立于本次）

- `prettier --check` 存在既有漂移文件（EventsPage、OutboundOrdersPage、SelectControl、StockDraftWorkspace、
  TemplateEditorDialog、guards.ts 及 AppPreferencesDialog 的模板单引号/CRLF），HEAD 即存在，本次未归一化，避免引入无关 churn。
- ESLint + typescript-eslint 未引入（方案 Phase 3 第 3 条列为独立决策项）。
