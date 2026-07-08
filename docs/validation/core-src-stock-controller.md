# `core/src/stock/controller/`

本目录定义库存 API 的 HTTP DTO、分页查询参数、响应体和 Axum handler。
`core/src/stock/controller.rs` 是入口和重新导出层，具体能力按业务拆到：

- `templates.rs`：模板字段、模板请求/响应和模板 handler。
- `items.rs`：库存物品请求/响应、分页查询参数和物品 handler。
- `inbound.rs`：入库单请求/响应、分页查询参数和入库 handler。
- `outbound.rs`：出库单请求/响应、分页查询参数和出库 handler。
- `dashboard.rs`：库存看板总览、趋势查询参数和看板 handler。
- `substitutes.rs`：替代料绑定请求、替代料响应和替代料 handler。
- `events.rs`：事件日志查询参数、响应体和事件日志 handler。
- `common.rs`：多个库存 HTTP 子模块共享的单据状态枚举和正数校验函数。

## `TemplateFieldType`

字段类型使用 Serde kebab-case 枚举，当前允许 `text`、`number`、`select`、`date`、`file` 和 `boolean`。服务层会把枚举转换为数据库稳定代码。

## `TemplateFieldDef`

校验入口：`ValidatedJson<TemplateCreateRequest>` 或 `ValidatedJson<TemplateUpdateRequest>`。`garde` 负责静态长度和未知字段拒绝，服务层负责字段组合规则。

| 字段 | 限制 |
| --- | --- |
| `field_name` | 必填；`garde length(min = 1, max = 64)`；trim 后非空；同一模板内大小写不敏感唯一 |
| `field_type` | 必填；只能是 `TemplateFieldType` 支持的枚举 |
| `required` | 可空；未传时服务层按 false 处理 |
| `searchable` | 可空；未传时服务层按 false 处理 |
| `options` | 仅 `select` 字段允许且必填；选项数量 1 到 128；选项 trim 后非空且大小写不敏感唯一 |
| `default_value` | 可空；存在时 trim 后非空，最大 256；`number` 必须能解析为有限数值，`boolean` 只允许 `true`/`false`，`select` 必须在选项内 |

## `TemplateCreateRequest`

校验入口：`ValidatedJson<TemplateCreateRequest>`。服务层会裁剪文本、检查名称唯一和模板字段组合规则。

| 字段 | 限制 |
| --- | --- |
| `name` | 必填；`garde length(min = 1, max = 128)`；trim 后非空；未软删除模板内唯一 |
| `description` | 可空；存在时 trim 后非空，最大 1024 |
| `fields` | 至少 1 个字段，最多 64 个字段；每个字段按 `TemplateFieldDef` 校验 |

## `TemplateUpdateRequest`

校验入口：`ValidatedJson<TemplateUpdateRequest>`。字段为空表示不修改；当前首版接口不通过 `null` 清空说明字段。

| 字段 | 限制 |
| --- | --- |
| `name` | 可不传；存在时 trim 后非空，最大 128；未软删除模板内唯一 |
| `description` | 可不传；存在时 trim 后非空，最大 1024 |
| `fields` | 可不传；存在时整体替换旧字段，并按 `TemplateFieldDef` 校验 |

## `TemplateCopyRequest`

| 字段 | 限制 |
| --- | --- |
| `name` | 必填；trim 后非空，最大 128；未软删除模板内唯一 |

## `TemplateResponse`

响应体返回模板基础资料和字段定义，不包含 `deleted_at`。软删除模板不会从详情和列表接口返回。

## `ItemCreateRequest`

校验入口：`ValidatedJson<ItemCreateRequest>` 在进入 handler 前执行 JSON 解析、未知字段拒绝和 `garde` 校验。服务层还会裁剪首尾空白并检查非负数值。

| 字段 | 限制 |
| --- | --- |
| `name` | 必填；`garde length(min = 1, max = 128)`；trim 后非空 |
| `sku` | 必填；`garde length(min = 1, max = 64)`；trim 后非空；未软删除记录唯一 |
| `category_id` | 可空；当前仅保存模板 ID，模板业务校验后续补齐 |
| `unit` | 必填；`garde length(min = 1, max = 32)`；trim 后非空 |
| `description` | 可空；存在时 trim 后非空，最大 1024 |
| `default_price` | 可空；服务层要求有限数字且非负 |
| `reorder_point` | 可空；服务层要求有限数字且非负 |

## `ItemUpdateRequest`

校验入口：`ValidatedJson<ItemUpdateRequest>`。字段为空表示不修改；当前首版接口不通过 `null` 清空可空字段。

| 字段 | 限制 |
| --- | --- |
| `name` | 可不传；存在时 trim 后非空，最大 128 |
| `sku` | 可不传；存在时 trim 后非空，最大 64；未软删除记录唯一 |
| `category_id` | 可不传；存在时更新模板关联 |
| `unit` | 可不传；存在时 trim 后非空，最大 32 |
| `description` | 可不传；存在时 trim 后非空，最大 1024 |
| `default_price` | 可不传；存在时服务层要求有限数字且非负 |
| `reorder_point` | 可不传；存在时服务层要求有限数字且非负 |

## `ItemListQuery`

查询参数不走 `ValidatedJson`。服务层负责默认值和边界：

- `page` 默认为 1，小于 1 时归一为 1。
- `page_size` 默认为 50，最大 200。
- `search` 存在时 trim 后不能为空。
- `category_id` 当前按精确模板 ID 筛选。

## `ItemResponse`

响应体包含物品基础资料和创建/更新时间，不包含 `deleted_at`。软删除物品不会从详情和列表接口返回。

## `OrderStatus`

出入库单状态响应枚举，使用 Serde snake_case。当前允许：

- `pending`：创建后待审批，未改变库存。
- `approved`：审批完成，已生成批次、库存流水和审计事件。
- `rejected`：已拒绝，不能再审批。

## `InboundItemRequest`

校验入口：`ValidatedJson<InboundCreateRequest>`。`garde` 负责静态字段限制，服务层负责物品存在性、数值有限性、模板扩展属性结构和字段类型校验。

| 字段 | 限制 |
| --- | --- |
| `item_id` | 必填；必须大于 0；服务层要求指向未软删除物品 |
| `quantity` | 必填；有限数字且大于 0 |
| `unit_price` | 必填；服务层要求有限数字且非负 |
| `location` | 可空；存在时 trim 后非空，最大 128 |
| `batch_no` | 可空；存在时 trim 后非空，最大 128 |
| `expires_at` | 可空；存在时 trim 后非空，最大 64 |
| `ext_attributes` | 可空；审批阶段必须是 JSON object；物品有关联模板时字段名必须来自模板，必填字段不能为空，字段值必须符合模板类型和选项；物品无模板时必须为空对象或不传 |

## `InboundCreateRequest`

校验入口：`ValidatedJson<InboundCreateRequest>`。创建阶段只保存 `pending` 单据和明细，不写库存批次或库存流水。

| 字段 | 限制 |
| --- | --- |
| `source` | 必填；`garde length(min = 1, max = 128)`；trim 后非空 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |
| `items` | 至少 1 条，最多 256 条；每条按 `InboundItemRequest` 校验 |

## `InboundListQuery`

查询参数不走 `ValidatedJson`。服务层负责默认值和边界：

- `page` 默认为 1，小于 1 时归一为 1。
- `page_size` 默认为 50，最大 200。
- `item_id` 存在时必须大于 0。
- `date_from` 和 `date_to` 存在时 trim 后不能为空；首版按 SQLite UTC 字符串格式做字典序筛选。

## `InboundResponse`

响应体包含入库单主表、状态、审批/拒绝时间和明细列表。创建返回 `pending`；审批返回 `approved`；拒绝返回 `rejected`。重复审批或拒绝非 `pending` 单据返回 `409 order_not_pending`。

## `OutboundItemRequest`

校验入口：`ValidatedJson<OutboundCreateRequest>`。`garde` 负责静态字段限制，服务层负责物品存在性和数值有限性；审批阶段由 repository 在事务内校验库存和批次可扣减性。

| 字段 | 限制 |
| --- | --- |
| `item_id` | 必填；必须大于 0；服务层要求指向未软删除物品 |
| `quantity` | 必填；有限数字且大于 0 |
| `batch_id` | 可空；存在时审批阶段只扣该批次，批次必须属于该物品且有足够剩余库存 |
| `location` | 可空；存在时 trim 后非空，最大 128 |

## `OutboundCreateRequest`

校验入口：`ValidatedJson<OutboundCreateRequest>`。创建阶段只保存 `pending` 单据和明细，不扣减库存。

| 字段 | 限制 |
| --- | --- |
| `destination` | 必填；`garde length(min = 1, max = 128)`；trim 后非空 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |
| `items` | 至少 1 条，最多 256 条；每条按 `OutboundItemRequest` 校验 |

## `OutboundListQuery`

查询参数不走 `ValidatedJson`。服务层负责默认值和边界：

- `page` 默认为 1，小于 1 时归一为 1。
- `page_size` 默认为 50，最大 200。
- `item_id` 存在时必须大于 0。
- `date_from` 和 `date_to` 存在时 trim 后不能为空；首版按 SQLite UTC 字符串格式做字典序筛选。

## `OutboundResponse`

响应体包含出库单主表、状态、审批/拒绝时间和明细列表。创建返回 `pending`；审批返回 `approved`；拒绝返回 `rejected`。重复审批或拒绝非 `pending` 单据返回 `409 order_not_pending`；库存不足或指定批次不可扣减返回 `409 insufficient_stock`。

## `DashboardOverviewResponse`

响应体为库存看板总览，只读当前有效库存和审批后产生的库存流水：

- `total_items` 统计未软删除库存物品数量。
- `total_quantity` 和 `total_value` 统计未软删除物品关联批次的当前剩余库存。
- `inbound_3d` 和 `outbound_3d` 只统计最近三天 `stock_movements` 中已审批流程写入的出入库流水。
- `slow_moving_items` 返回当前有库存且 30 天内无出入库流水的物品。

## `TrendsQuery`

查询参数不走 `ValidatedJson`。服务层负责默认值和边界：

- `days` 默认为 30。
- `days` 小于 1 时按 1 处理。
- `days` 大于 365 时按 365 处理。

## `TrendsResponse`

响应体按日期升序返回每日入库/出库数量。无流水日期也会返回数量 0，便于前端直接绘制连续趋势图。

## `SubstituteItem`

校验入口：`ValidatedJson<SubstituteBindRequest>`。`garde` 负责静态字段限制，服务层和 repository 负责物品存在性、去重和循环绑定检测。

| 字段 | 限制 |
| --- | --- |
| `substitute_item_id` | 必填；必须大于 0；不能等于主物品 ID；必须指向未软删除物品 |
| `priority` | 必填；必须大于 0；同一次提交内不能重复 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |

## `SubstituteBindRequest`

校验入口：`ValidatedJson<SubstituteBindRequest>`。提交列表会整体替换该物品已有替代料关系；空列表表示清空当前物品的替代料。

## `SubstituteDetailResponse`

响应体包含主物品 ID、替代料 ID、替代料名称、当前库存量、优先级、备注、创建人和创建时间。软删除主物品或替代物品不会出现在替代料列表中。

## `EventListQuery`

查询参数不走 `ValidatedJson`。服务层负责默认值和边界：

- `page` 默认为 1，小于 1 时归一为 1。
- `page_size` 默认为 50，最大 200。
- `entity_type`、`action`、`date_from` 和 `date_to` 存在时 trim 后不能为空。
- `entity_id` 和 `user_id` 存在时必须大于 0。

## `EventLogResponse`

响应体包含审计事件 ID、时间、操作人、实体、动作和 JSON 详情。`details_json` 缺失或无法解析时，响应 `details` 为 JSON null；`audit_events.user_id` 为空时，`username` 也为空。
