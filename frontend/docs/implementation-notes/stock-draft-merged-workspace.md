# 出入库合并草稿工作台

状态：2026-07-26 实现、验证并于同日正式切换——`/inbound`、`/outbound` 直接指向合并页，
旧的 `InboundDraftPage.vue` / `OutboundDraftPage.vue` 与 `InboundDraftStep.vue` 已删除，
评估期 `/draft/*` 路由已移除。本文保留为设计与切换记录。

## 目标与形态

把"新建入库"与"新建出库"逐字级重复的交互骨架收敛为一份实现，同时保持两域的
行编辑语义（入库固定字段 vs 出库批次分配+成本估算）各自独立，不建 kind 判别的巨型编辑器。

结构：

- `pages/StockDraftPage.vue`：合并页，按路由 `kind` prop 装配入库或出库领域；
  单根条件链渲染（路由出口的 `Transition` 要求单根，多根会导致切换后空白）。
- `components/stock-draft/StockDraftWorkspace.vue`：泛型工作台壳（`generic="L"`），
  拥有页头/摘要槽、单据头、明细表骨架（中间列 `#line-cells` 槽）、行编辑 Dialog 壳
  （内容 `#line-editor` 槽、固定"暂存并关闭/完成并继续添加"）、物品选择流、
  清空/离开/提交三态确认、路由离开守卫、Esc 与 `#extras` 附加 Dialog 槽。
- `pages/stock-draft/flow.ts`：壳与领域装配之间的 `StockDraftFlow`/`StockDraftTexts`
  契约与 `StockDraftWorkspaceHandle` 反向桥（装配经 handle 调用壳的编辑器/选择器）。
- `pages/stock-draft/useInboundDraft.ts`、`useOutboundDraft.ts`：领域装配，
  复用旧页的 model 与持久化 composable（同一 localStorage 键，草稿与旧页互通），
  出库侧完整移植分配草稿、批次分页与成本估算。
- `components/stock-draft/OutboundAllocationEditor.vue`:出库行编辑内容（从旧页抽取）；
  入库行编辑直接复用现有 `InboundLineEditor.vue`。

## 路由与样式

- 正式路由 `/inbound`（`inbound`）与 `/outbound`（`outbound`）以 `props.kind`
  指向合并页，权限与标题沿用 `getAppRouteMeta`；路由出口按路由名重建实例。
- 域样式已迁移为 `pages/stock-draft/inbound.scss`、`outbound.scss`（壳用 inbound-\*
  骨架类、出库域用 outbound-\* 类，类名沿用原前缀未重命名，属可选的后续整理项）。

## 与旧页的有意对齐差异

- 出库行编辑 Dialog 的上下文头补齐了物品主图（与入库一致；旧出库页无图）。
- 清空/离开/提交确认统一为三态 `confirmMode` 实现（旧入库页为两个独立 ref）。
- 物品选择器加载错误文案统一为入库页口径（403 → "当前账号没有读取物品的权限"）。
- 入库提交模式在提交时按 `stock.inbound.approve` 权限决定 direct/pending，
  与旧页 `defaultSubmissionMode` 行为等价。
- 保留旧出库页的 `canReadItems` 拦截口径（检查 outboundCreate；路由已要求该权限，
  实际不可达，评估期按原样保留待后续澄清）。

## 已验证（chrome-devtools MCP，2026-07-26）

- 双 kind 首载与互相切换渲染正常（单根修复后），控制台零报错。
- 草稿互通：旧页产生的 v6 入库草稿 / v1 出库草稿在合并页原样恢复、编辑与保存。
- 出库全流程：分配编辑（FIFO/指定批次、批次列表分页）、完成后自动续选、
  直接出库成功（单号 #3，批次 IN-10-13 扣减）。
- 入库行编辑、摘要与提交确认渲染正常（未重复提交入库单）。
- 桌面 1440×900 与窄视口无横向溢出，明细表按标签卡片降级。

## 切换记录（2026-07-26）

1. 已完成：`/inbound`、`/outbound` 指向合并页；删除 `InboundDraftPage.vue`、
   `OutboundDraftPage.vue`、`InboundDraftStep.vue`；`useInboundItemCatalog`
   更名 `useStockItemCatalog`；评估期 `/draft/*` 路由移除。
2. 已完成：SCSS 迁移至 `pages/stock-draft/` 归属；类名保留原前缀（可选整理项）。
3. 已完成：routes.md、代码地图与相关页面文档同步。
