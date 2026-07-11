# WineStock 业务 API 实施方案

本文档记录 `core/docs/business-api.md` 入口及 `core/docs/business-api/` 拆分文档对应的首版业务核心 API 落地方案。
它是实施计划，不直接改变项目架构约束；实施时仍以 `docs/architecture.md`、`docs/runtime-networking.md`、`docs/platforms.md`、`docs/project-structure.md`、`docs/agent-checklist.md` 和 `docs/code-map.md` 为准。

## 目标和范围

按 `core/docs/business-api.md` 与 `core/docs/business-api/` 落地首版业务核心 API，范围限定在 `core` 共享 Axum 服务库、SQLite 持久化、RBAC、OpenAPI 和文档同步，不涉及桌面、Android、WebView 或前端打包。

业务默认规则：

- 删除策略：软删除优先。
- 出入库流程：简单审批，创建单据为 `pending`，审批通过才改变库存。
- 库存扣减和估值：未指定批次时 FIFO 扣减，看板库存价值使用加权平均成本。
- Migration 策略：不新增 migration 文件，直接扩展现有 `m20260706_000001_initial_schema.rs` 初始 schema。

## 主要改动

- 新增 `core/src/stock/` 领域模块：
  - `mod.rs` 注册 stock 路由和权限中间件。
  - `controller.rs` 承载 Axum handler 与 `utoipa::path`。
  - `service.rs` 承载业务流程、事务边界和错误映射。
  - `permissions.rs` 定义库存、审批、替代料和审计权限。
- 新增 stock/audit entity 与 repository：
  - 表前缀使用 `stock_` 与 `audit_`。
  - repository 暴露业务方法，handler 不直接拼 SQL 或组合 SeaORM 查询。
  - 出入库审批必须在单个数据库事务内完成。
- 修改现有初始 migration：
  - 直接在 `core/src/persistence/migration/m20260706_000001_initial_schema.rs` 的 `INITIAL_SCHEMA` 增加业务表和索引。
  - 同步在 `DROP_SCHEMA` 按依赖反序删除新增索引和表。
  - 不创建 `m20260708_000002_*` 或其他新增 migration。
- 更新 HTTP、OpenAPI 和 RBAC：
  - `core/src/http/router.rs` merge `stock::router(state.clone())`。
  - `core/src/http/docs.rs` 纳入新 DTO、响应体和 endpoint。
  - `core/src/rbac/bootstrap.rs` 补齐新权限和角色权限关系。
- 更新项目文档：
  - `core/docs/business-api.md` 作为入口，`core/docs/business-api/` 记录已实现接口、审批 endpoint、状态枚举和软删除规则。
  - `docs/code-map.md`、`core/docs/database-schema.md`、`core/docs/rbac-permission-model.md`、`core/docs/validation/*` 同步更新。

## HTTP API 和类型

保留现有 auth/users API 和 core 启动 API 不变。

新增 endpoint：

- Items：`POST /api/items`、`GET /api/items`、`GET /api/items/{id}`、`PUT /api/items/{id}`、`DELETE /api/items/{id}`。
- Templates：`POST /api/templates`、`GET /api/templates`、`GET /api/templates/{id}`、`PUT /api/templates/{id}`、`DELETE /api/templates/{id}`、`POST /api/templates/{id}/copy`。
- Inbound：`POST /api/inbound`、`GET /api/inbound`、`GET /api/inbound/{id}`。
- Outbound：`POST /api/outbound`、`GET /api/outbound`、`GET /api/outbound/{id}`。
- Stock approvals：`POST /api/stock-approvals/inbound/{id}/approve`、`POST /api/stock-approvals/inbound/{id}/reject`、`POST /api/stock-approvals/outbound/{id}/approve`、`POST /api/stock-approvals/outbound/{id}/reject`。
- Dashboard：`GET /api/dashboard/overview`、`GET /api/dashboard/trends`。
- Substitutes：`GET /api/substitutes`、`GET /api/substitutes/{item_id}`、`PUT /api/substitutes/{item_id}`、`DELETE /api/substitutes/{item_id}/{substitute_item_id}`。
- Events：`GET /api/events`。

新增通用响应：

- `PaginatedResponse<T>`：`items`、`total`、`page`、`page_size`、`total_pages`。
- 分页默认：`page = 1`，`page_size = 50`，最大 `200`。

新增单据状态：

- `pending`：待审批，未影响库存。
- `approved`：已审批，库存变动已入账。
- `rejected`：已拒绝，不能再审批。

DTO 统一使用：

- `#[serde(deny_unknown_fields)]`
- `garde::Validate`
- `utoipa::ToSchema`

## 数据库和事务

直接扩展初始 schema，新增表：

- `stock_templates`
- `stock_template_fields`
- `stock_items`
- `stock_inbound_orders`
- `stock_inbound_order_items`
- `stock_outbound_orders`
- `stock_outbound_order_items`
- `stock_batches`
- `stock_movements`
- `stock_substitutes`
- `audit_events`

关键约束：

- `stock_items.sku` 对未删除记录唯一。
- `stock_templates.name` 对未删除记录唯一。
- 软删除表使用 `deleted_at TEXT NULL`。
- 数量、价格、库存余额必须非负，出入库明细数量必须大于 `0`。
- 单据状态使用 SQLite `CHECK`。
- 替代料禁止自引用并禁止重复替代物品或重复优先级。

入库事务：

- 创建只写 `pending` 单据和明细，不增加库存。
- 审批校验状态和模板扩展属性，生成或增加批次库存，写库存流水和审计事件。
- 拒绝后状态改为 `rejected`，不能再审批。

出库事务：

- 创建只写 `pending` 单据和明细，不扣库存。
- 审批校验库存；指定批次扣指定批次，未指定批次按 FIFO 扣减。
- FIFO 排序规则：`expires_at ASC NULLS LAST, received_at ASC, id ASC`。
- 库存不足返回 `409`，事务整体回滚。
- 拒绝后状态改为 `rejected`，不能再审批。

审计事件：

- 创建、更新、删除、审批、拒绝、替代料关系变更和删除关系写 `audit_events`。
- `details` 保存 JSON 差异或关键输入摘要。
- 不记录 JWT、密码、refresh token、签名密钥等敏感值。

## 实施顺序

1. 基础设施切片：
   - 添加通用分页 DTO、stock 错误类型、stock 权限常量。
   - 添加业务表到现有初始 migration。
   - 添加 SeaORM entity、repository 和模块声明。
   - 更新 RBAC bootstrap 并覆盖权限初始化测试。
2. 物品和模板：
   - 实现模板 CRUD/copy 和字段校验。
   - 实现物品 CRUD、软删除、SKU 唯一、列表筛选。
   - 同步 OpenAPI 和文档。
3. 入库：
   - 实现入库创建、列表、详情、审批、拒绝。
   - 审批时校验模板扩展属性并生成批次和库存流水。
4. 出库：
   - 实现出库创建、列表、详情、审批、拒绝。
   - 审批时实现指定批次扣减和 FIFO 自动扣减。
5. 派生能力：
   - 实现看板 overview/trends。
   - 实现替代料整体替换、查询、删除关系和循环绑定检测。
   - 实现事件日志查询。
6. 收尾：
   - 更新 `docs/code-map.md`、`core/docs/database-schema.md`、`core/docs/business-api.md`、`core/docs/business-api/`、`core/docs/rbac-permission-model.md`、`core/docs/validation/*`。
   - 全量注释审计：搜索 `//`、`///`、`//!`、`/*`，确保变更源文件中的说明性注释为中文且不陈旧。

## 测试计划

- Migration：
  - 初始 schema 一次创建所有 auth/storage/stock/audit 表。
  - 重复执行 migration 幂等。
  - `DROP_SCHEMA` 能清理新增业务表。
- RBAC：
  - 启动补齐新权限。
  - `admin/staff/viewer` 权限关系符合计划。
  - 无权限 token 调用受保护业务 API 返回 `403`。
- Items/Templates：
  - CRUD、软删除、列表筛选、SKU/name 冲突、关联删除冲突。
- Inbound/Outbound：
  - 创建 pending 不改变库存。
  - approve 后库存、流水、审计事件正确。
  - reject 后不能 approve。
  - FIFO、指定批次、库存不足回滚均覆盖。
- Dashboard/Substitutes/Events：
  - 看板统计与已审批流水一致。
  - 替代料禁止自引用、重复替代物品、重复优先级和循环绑定。
  - 事件日志分页和筛选正确。

验证命令：

```text
cargo +stable fmt --all -- --check
cargo +stable check --workspace --all-targets
cargo +stable test --workspace
cargo +stable build -p winestock-server
cargo run -p winestock-server
```

启动 server 后检查：

```text
/api-docs/openapi.json
/swagger-ui
```

## 假设

- 因项目仍处于初始 schema 阶段，允许直接修改原有 migration；不考虑兼容已有生产数据库。
- 不引入新依赖。
- 金额和数量首版使用现有依赖可支持的数值类型；如需财务级 decimal，后续单独决策。
- 首版不做多仓库模型，`location` 作为文本库位。
- 所有业务能力属于 `core`，server shell 只负责启动共享服务。
- 新增和修改的代码注释、公共类型文档注释按项目规则使用中文。
