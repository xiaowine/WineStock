# 删除入库模板评估

状态：2026-07-26 评估完成；同日用户确认按方案 B 实施，代码/测试/文档已完成，测试库按原地转换处理。本文保留为跨组件影响评估与决策记录。

## 背景与动机

- `/templates` 页面并存三类概念：物品分类、物品属性模板、入库模板，概念区分成本高，2026-07 讨论中确认三者易混淆。
- 实际使用率极低：当前开发测试库中 3 套入库模板均为 bootstrap 种子（元器件收货/耗材收货/通用收货），13 条入库明细中仅 2 条真正填写过模板属性值，收货文件绑定 0 条。
- 此前已决定入库模板不参与"全局默认"功能（依赖"属性模板 default_inbound_template_id → 推荐链"机制）；删除入库模板将连推荐链机制一并移除。

## 现状：入库模板是什么

- 数据本体两张表：`stock_inbound_templates`（软删除、活跃名称唯一）与 `stock_inbound_template_fields`（字段类型 text/number/select/date/file/url/boolean，required/searchable/options_json/default_value/sort_order）。
- 行为：定义"单次收货状态"的动态属性字段（包装状态、质检结果、收货照片等），与物品属性模板（物品长期属性 + 目录展示）刻意分职，不写回物品主数据。
- 入库单引用方式：
  - `stock_inbound_order_items.inbound_template_id`：可空 FK（ON DELETE SET NULL），记录来源，非快照。
  - `stock_inbound_order_item_attributes`：实际收货属性**快照**（field_name/field_type/value_json/unit 均为拷贝），另带可空溯源 FK `template_field_id`。
  - **`searchable` 标志未被快照**，只存在模板字段表上——入库/出库历史的 `template:*` 分面筛选 SQL 结构性 JOIN 模板字段表（`stock_repo/search.rs` 的 `inbound_template_filter_values_sql` / `outbound_template_filter_values_sql`）。
- 推荐链：物品属性模板 `default_inbound_template_id` → `/api/items/options` 投影 `recommended_inbound_template_id/_available` → 前端预选；服务端创建入库时同样兜底推导（`core/src/stock/service/inbound.rs:76-85`）。
- 入库单模板本就可选：无模板时 `ext_attributes` 必须为空（service 校验拒绝）。即**收货属性录入完全由模板驱动**。

## 核心结论：删除范围存在一个前置决策

"删入库模板"不是删两张表：入库单的 `ext_attributes`（收货属性）特性整体由模板驱动，必须二选一。

### 方案 A：仅删模板，收货属性改为自由录入

- 保留 `stock_inbound_order_item_attributes`；`validate_create_attributes`/`validate_inbound_attributes` 重写为无 schema 校验；`searchable` 需快照进属性表，否则 `template:*` 分面失效；前端 `InboundLineEditor` 需重新设计为逐行自建字段（类似物品自定义属性）。
- 实质是**删除 + 新功能开发**，不是净删除；且"逐行自由属性"仍是一个新概念，与消除概念混淆的动机相悖。

### 方案 B：连收货属性一起删（推荐）

- 额外删除：`stock_inbound_order_item_attributes`、`storage_inbound_file_bindings` 两张表，入库 DTO 的 `inbound_template_id`/`ext_attributes`，`service/inbound.rs` 全部属性与文件校验（含审批期重校验），入库/出库历史 `template:*` 分面，前端属性录入与展示块。
- 入库表单收敛为固定字段：数量/单价/库位/批次号/有效期。
- 与实际使用一致（历史仅 2 条属性值、0 条文件绑定），是净删除。

推荐方案 B：使用率、概念收敛、方案 A 的新开发成本都指向 B；若未来重新需要"收货态记录"，按新需求重新设计比背着旧模型改造干净。

## 删除后的功能损失（方案 B 口径）

1. 收货态动态属性：七类字段录入、默认值预填、逐字段校验、收货照片绑定（实际从未使用）。
2. 入库/出库历史的 `template:*` 属性分面筛选（出库侧经批次反查入库明细实现）；基础分面（状态、去向等）不受影响。
3. 推荐链与 `/api/items/options` 的两个推荐字段（该接口出库选品页共用，契约同步变更）。
4. 历史入库单详情/审批详情中的"模板 #id"行与属性快照展示（现有 2 条快照随表删除）。

收益：模板页三 tab → 两 tab、三概念 → 两概念；`InboundDraftPage` 约 40% 模板编排代码、core 五个整文件及散布约 20 个文件的模板分支全部移除。

## 影响面清单

### core（Rust）

整文件删除（5）：

- `persistence/entity/inbound_template.rs`、`persistence/entity/inbound_template_field.rs`
- `stock/service/templates/inbound.rs`、`stock/controller/templates/inbound.rs`、`persistence/repository/stock_repo/templates/inbound.rs`

手术式修改（约 20 文件）：

- 路由/权限：`stock/mod.rs`（3 条 `/inbound-templates*` 路由、`INBOUND_TEMPLATE_READ_PERMISSIONS`、`inbound_template_read`）。
- 门面再导出：`stock/controller.rs`、`stock/service.rs`、`controller/templates/mod.rs`、`service/templates/mod.rs`、`repository/mod.rs`、`stock_repo.rs`、`stock_repo/templates/mod.rs`、`entity/mod.rs`。
- 共享模块内的入库分支：`service/templates/common.rs`（`inbound_template_response`/`inbound_fields`）、`service/templates/item.rs`（`ensure_inbound_template` 校验）、`repo templates/common.rs`（6 个入库专用函数）、`types/templates.rs`（`CreateInboundTemplate`/`UpdateInboundTemplate`/`InboundTemplateDetail`）。
- 推荐链：`entity/item_attribute_template.rs`、`controller/templates/item.rs`、`repo items.rs`（options SQL 的 LEFT JOIN 与两列）、`types/items.rs`、`service/response.rs`、`controller/items.rs`。
- 入库主流程：`service/inbound.rs`（兜底推导 + 属性校验 + 审批重校验）、`controller/inbound.rs`、`repo inbound.rs`、`types/inbound.rs`、`service/error.rs`（`InboundTemplateInvalid`）。
- 分面搜索：`repo search.rs`（`inbound_template_filter_values_sql`、`outbound_template_filter_values_sql` 及调用点）。
- OpenAPI：`http/docs.rs`（6 条路径 + 3 个 schema 注册）。
- bootstrap：`stock/bootstrap/specs.rs`（`DEFAULT_INBOUND_TEMPLATES` 三套、`TemplateSpec`、`ItemTemplateSpec.default_inbound_template_name`）、`bootstrap/mod.rs`（播种循环与按名配对块）；分类与物品属性模板播种不受影响。

保留勿动（与物品属性模板共享）：`normalize_template_fields`、`audit_template_change`、`TemplateFieldInput`、`TemplateFieldType/Def/Response/CopyRequest`、权限 `stock.template.read/manage`（无任何权限常量需要删除）。

事件：entity_type `inbound_template` 随仓储文件删除消失；events 文档与前端事件目录同步。

### 数据库（唯一迁移 `m20260706_000001_initial_schema.rs` 原地修改）

- 删表 4（方案 B）：`stock_inbound_templates`、`stock_inbound_template_fields`、`stock_inbound_order_item_attributes`、`storage_inbound_file_bindings`（含各索引与 down 分支）。方案 A 只删前两张。
- 删列 2（含 FK）：`stock_item_attribute_templates.default_inbound_template_id`、`stock_inbound_order_items.inbound_template_id`。
- 迁移源与现测试库已有漂移（迁移源 `stock_inbound_template_fields` 含 `catalog_visible` 列，现库无）；随表删除自然消除。

### frontend（Vue）

整文件删除（1）：`src/api/inboundTemplates.ts`。

手术式修改（约 10 文件）：

- 入库草稿流：`InboundDraftPage.vue`（约 40%：候选加载/缓存/请求竞态版本/失效侦测/切换确认 Dialog 等编排）、`pages/inbound-draft/model.ts`（行状态 6 个模板字段、`templateFieldError`、`ext_attributes` 构造）、`pages/inbound-draft/presentation.ts`（`template_not_found` 文案）、`composables/useInboundDraftPersistence.ts`（存储键 v5→v6，清理模板字段草稿与 IndexedDB 草稿图片孤儿）、`components/inbound/InboundLineEditor.vue`（整个动态字段渲染区，保留数量/单价/库位/批次/有效期）、`components/inbound/InboundDraftStep.vue`（"模板/批次"列改"批次"、模板摘要与重试块）。
- 模板页：`TemplatesPage.vue`（第三 tab、物品模板"默认入库模板"列）、`pages/templates/model.ts`、`components/templates/TemplateEditorDialog.vue`（默认入库模板选择块、kind 联合收窄）、`TemplateDeleteDialog.vue`（入库文案分支）。
- 历史展示：`InboundOrdersPage.vue`、`components/approvals/InboundApprovalDetails.vue`（模板 `#id` 行；方案 B 下属性快照块一并移除）。
- 事件目录：`pages/events/catalog.ts` 删 `inbound_template` 项（`source_template_id` 为复制通用字段，保留）。
- `api/inbound.ts`（再导出行）；`api/generated/schema.d.ts` 由 `pnpm gen:api` 重新生成。

路由与导航不动：`/templates` 保留（仍有分类 + 物品属性模板），无入库模板专属路由或菜单项。

### 测试

- core 整删：入库模板 CRUD/复制/坏字段专项（`stock_attribute_templates.rs` 部分）、`stock_items.rs` 的 options 推荐状态用例。
- core 重写：共享 fixture `seed_template_bound_item`（`stock_inbound.rs`）、`seed_search_item`（`stock_outbound.rs`，其 `template:brand` 出库分面断言随功能删除）、`files.rs` 入库文件绑定用例、`persistence_connection.rs` schema 断言。
- core 平凡改：`stock_dashboard.rs`、`stock_substitutes.rs`、`stock_locations.rs`、`stock_events.rs` 仅删 fixture 字段行。
- frontend：仅 `tests/itemLcscMapping.test.mjs` 一处非断言 mock 字段，可选清理。

### 文档（约 25 处）

- core/docs：`business-api/templates.md`、`inbound.md`、`outbound.md`、`items.md`、`permissions.md`、`events.md`、`database-schema.md`、`validation/` 三份。
- docs/code-map：`core/http-api.md`、`core/stock.md`、`core/persistence.md`、`frontend.md`。
- frontend/docs：`page-templates.md`、`page-inbound.md`、`page-inbound-orders.md`、`page-stock-approvals.md`、`page-events.md`、`routes.md`、`README.md`；`implementation-notes/inbound-outbound-item-operation-flow-remediation.md` 订正、`inbound-template-usability-remediation.md` 整份删除或标注废弃。
- 根 `TODO.md`："物品导入向导/扫码（模板问题）"若指入库模板需同步改写（见待决策项 3）。

## 测试库数据处理

`target/debug/data/winestock.sqlite` 为活跃开发库。沿用统一属性定义重构先例：停服、备份、sqlite3 事务内原地转换。本次转换量小：删 4 表（合计 17 行）+ 2 列（SQLite 删带 FK 的列按 12 步表重建执行，或验证 `DROP COLUMN` 可行后直用），10 张入库单与 13 条明细全部保留；转换后修正 `seaql_migrations` 并执行 `PRAGMA foreign_key_check`。也可直接重置测试库（丢弃现有单据），需用户确认。

## 工作量与风险

- 规模：core 5 整删 + 约 20 手术文件 + 迁移；frontend 1 整删 + 约 10 手术文件 + `gen:api`；约 10 个测试文件；约 25 处文档。中大型一次性重构，建议单独分支一次完成，删除类改动不留中间态。
- 顺序建议：migration/entity → repository → service → controller/OpenAPI → `pnpm gen:api` → frontend → 测试 → 文档/代码地图 → 测试库转换 → smoke（server 启动、OpenAPI、入库全流程、模板页、审批、历史筛选）。
- 主要风险：
  1. ext_attributes 去留未决策就动手（本文推荐方案 B，待确认）。
  2. 误删与物品属性模板共享的代码（见"保留勿动"清单）。
  3. 分面搜索 SQL 与出库测试的隐性耦合（已定位，按清单处理）。
  4. 本地草稿兼容：`storageKey` 不升版会静默恢复孤儿模板字段草稿。

## 待决策项

1. 方案 A / 方案 B（推荐 B）。
2. 测试库：原地转换保留现有单据（默认沿用先例）还是重置。
3. `TODO.md` "物品导入向导/扫码（模板问题）"中的"模板"是否指入库模板；若未来导入向导规划依赖它，应先明确向导需求再定删除。
