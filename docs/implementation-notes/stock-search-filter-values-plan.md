# 库存搜索和筛选值 API 实现方案

本文档记录库存物品列表、入库历史列表和出库历史列表的搜索、筛选值接口设计与实现约束。正式接口说明以 `docs/business-api.md`、OpenAPI 标注和 `docs/code-map.md` 为准。

## 目标

新增和增强六个 API：

| API | 类型 | 目标 |
| --- | --- | --- |
| `GET /api/items?search=` | 增强现有接口 | 搜索物品列表，结果仍按物品去重返回。 |
| `GET /api/items/filter-values` | 新增接口 | 返回当前库存视角下可用于物品列表筛选的字段和值。 |
| `GET /api/inbound?search=` | 增强现有接口 | 搜索入库历史，结果仍按入库单返回。 |
| `GET /api/inbound/filter-values` | 新增接口 | 返回入库历史视角下可用于入库列表筛选的字段和值。 |
| `GET /api/outbound?search=` | 增强现有接口 | 搜索出库历史，结果仍按出库单返回。 |
| `GET /api/outbound/filter-values` | 新增接口 | 返回出库历史视角下可用于出库列表筛选的字段和值。 |

这些接口都属于 `core` 的库存业务 API，按资源使用细分只读权限，不涉及平台 shell、前端打包或网络绑定行为。

## 数据语义

模板字段定义存储在 `stock_template_fields`。它定义字段名、字段类型、是否可搜索、选项和默认值。

入库实际填写的模板值存储在 `stock_inbound_order_items.ext_attributes_json`。它是 JSON object，键是模板字段名，值是本次入库填写的实际内容。

当前库存视角以 `stock_batches.remaining_quantity > 0` 为准。要从当前库存追溯模板值，需要经过：

```text
stock_batches
  -> stock_batches.inbound_order_item_id
  -> stock_inbound_order_items.ext_attributes_json
```

入库历史视角以 `stock_inbound_orders` 和 `stock_inbound_order_items` 为准。它不受当前库存余额影响，已经出完的历史批次仍然属于入库历史。

出库历史视角以 `stock_outbound_orders` 和 `stock_outbound_order_items` 为准。出库明细没有直接保存模板实际值；已指定批次或已审批产生库存流水的明细，可通过批次反查到入库明细的批次号、有效期和模板实际值。

## 通用响应结构

筛选值接口统一返回字段列表。字段值中的 `count` 含义跟接口目标一致：

- `GET /api/items/filter-values`：`count` 是命中该值的去重物品数量。
- `GET /api/inbound/filter-values`：`count` 是命中该值的去重入库单数量。
- `GET /api/outbound/filter-values`：`count` 是命中该值的去重出库单数量。

建议 DTO：

```json
{
  "fields": [
    {
      "key": "template:品牌",
      "label": "品牌",
      "source": "template",
      "value_type": "text",
      "values": [
        { "value": "ST", "count": 12 },
        { "value": "TI", "count": 7 }
      ]
    }
  ]
}
```

字段说明：

| 字段 | 说明 |
| --- | --- |
| `key` | 前端使用的稳定筛选字段 key。内置字段使用 `base:*`，模板字段使用 `template:*`。 |
| `label` | 展示名称。 |
| `source` | `base` 或 `template`。 |
| `value_type` | `text`、`number`、`select`、`date`、`file`、`url`、`boolean` 或 `mixed`。同名模板字段跨模板类型不一致时使用 `mixed`。 |
| `values` | 当前视角下出现过的值和计数，按 `count DESC, value ASC` 排序。 |

所有返回值统一转成字符串。数字、布尔值和日期由后端按稳定文本格式输出；`null`、空字符串、JSON object 和 JSON array 不进入筛选值。

## `GET /api/items?search=`

### 业务语义

搜索当前物品列表。这个接口的结果单位是物品，所以一个物品即使命中多条批次或多条入库明细，也只能返回一次。

已有分页参数、`category_id` 参数保持兼容。

### 搜索范围

基础物品字段：

- `stock_items.name`
- `stock_items.sku`
- `stock_items.unit`
- `stock_items.description`

关联模板字段：

- `stock_templates.name`
- `stock_templates.description`

当前库存中的模板实际值：

- 来源为 `stock_batches.remaining_quantity > 0` 关联到的 `stock_inbound_order_items.ext_attributes_json`
- 搜索 JSON object 的值，不搜索 JSON 字段名
- 搜索所有标量值，不只限于 `searchable = true`

不搜索：

- 数据库主键和外键，例如 `id`、`category_id`、`item_id`、`order_id`、`batch_id`
- 创建、更新、删除时间
- 库存数量、价格、余额等数值业务字段
- JSON object、array 和 null

### 查询策略

仓储层应使用 `EXISTS` 子查询判断模板值是否命中，避免 join 后把物品重复放大。

搜索词在服务层继续执行 trim 和空值校验，仓储层统一使用小写模糊匹配：

```text
lower(value_text) LIKE lower('%keyword%')
```

SQLite JSON 处理建议使用 `json_each(ext_attributes_json)`，并用 `json_valid` 防御损坏数据。

## `GET /api/items/filter-values`

### 业务语义

返回物品列表筛选项。接口不接收参数，不要求前端传 `template_id` 或 `field_name`。

这个接口是当前库存视角，只统计仍有库存的批次：

```text
stock_batches.remaining_quantity > 0
```

如果某个字段值只出现在已出完的历史批次里，它不应该出现在该接口返回值中。

### 字段范围

内置字段首版建议返回：

- `base:category`：物品关联模板名称
- `base:unit`：物品计量单位
- `base:location`：当前库存批次对应入库明细中的库位

模板字段：

- 只统计 `stock_template_fields.searchable = true` 的字段
- 字段值来自当前库存批次关联的 `ext_attributes_json`
- 不要求模板 ID，跨模板同名字段全局合并

不返回高基数字段作为首版筛选项，例如物品名称、SKU、批次号和 URL。它们更适合通过 `search` 搜索；如果前端后续明确需要，可以作为单独字段加入。

### 计数规则

`count` 表示拥有该字段值的去重物品数量。

同一物品多个当前批次都含有同一个值时，只计 1 次。同一物品确实拥有多个不同值时，可以分别计入多个值。

## `GET /api/inbound?search=`

### 业务语义

搜索入库历史。这个接口的结果单位是入库单，所以一张入库单即使命中多条明细，也只能返回一次。

现有分页参数、`item_id`、`date_from` 和 `date_to` 保持兼容。新增 `search` 后，时间和物品筛选仍应与搜索条件取交集。

### 搜索范围

入库单主表字段：

- `stock_inbound_orders.source`
- `stock_inbound_orders.notes`
- `stock_inbound_orders.status`

入库明细字段：

- `stock_inbound_order_items.location`
- `stock_inbound_order_items.batch_no`
- `stock_inbound_order_items.expires_at`

关联物品字段：

- `stock_items.name`
- `stock_items.sku`
- `stock_items.unit`
- `stock_items.description`

模板实际值：

- `stock_inbound_order_items.ext_attributes_json` 中的所有标量值
- 搜索 JSON object 的值，不搜索 JSON 字段名
- 包含 `pending`、`approved` 和 `rejected` 单据中的原始入库填写值

不搜索数据库主键、外键、创建/更新时间、数量和单价。

### 查询策略

入库搜索也应使用 `EXISTS` 或按订单 ID 去重，避免一张单据有多条明细时重复出现在分页结果中。

如果 `search` 存在，计数查询和数据查询必须使用同一套过滤条件，保证 `total` 与当前页一致。

## `GET /api/inbound/filter-values`

### 业务语义

返回入库历史列表筛选项。接口不接收参数，不要求前端传模板 ID 或字段名。

这个接口是历史单据视角，不受当前库存余额影响。即使某批库存已经出完，只要历史入库记录里出现过，对应字段值就可以作为入库历史筛选值。

### 字段范围

内置字段首版建议返回：

- `base:source`：入库来源
- `base:status`：入库单状态
- `base:item`：物品名称
- `base:sku`：物品 SKU
- `base:location`：入库库位
- `base:batch_no`：外部批次号

模板字段：

- 只统计 `stock_template_fields.searchable = true` 的字段
- 字段值来自入库明细的 `ext_attributes_json`
- 不要求模板 ID，跨模板同名字段全局合并

### 计数规则

`count` 表示拥有该字段值的去重入库单数量。

同一入库单的多条明细都含有同一个值时，只计 1 次。同一入库单确实含有多个不同值时，可以分别计入多个值。

## `GET /api/outbound?search=`

### 业务语义

搜索出库历史。这个接口的结果单位是出库单，所以一张出库单即使命中多条明细或多条扣减流水，也只能返回一次。

现有分页参数、`item_id`、`date_from` 和 `date_to` 保持兼容。新增 `search` 后，时间和物品筛选仍应与搜索条件取交集。

### 搜索范围

出库单主表字段：

- `stock_outbound_orders.destination`
- `stock_outbound_orders.notes`
- `stock_outbound_orders.status`

出库明细字段：

- `stock_outbound_order_items.location`

关联物品字段：

- `stock_items.name`
- `stock_items.sku`
- `stock_items.unit`
- `stock_items.description`

批次和模板实际值：

- 指定批次明细通过 `stock_outbound_order_items.batch_id` 关联 `stock_batches`
- 审批后按 FIFO 扣减的明细通过 `stock_movements.outbound_order_item_id` 和 `stock_movements.batch_id` 关联 `stock_batches`
- 批次号和有效期来自 `stock_batches.batch_no`、`stock_batches.expires_at`
- 模板实际值通过 `stock_batches.inbound_order_item_id` 反查 `stock_inbound_order_items.ext_attributes_json`
- 搜索 JSON object 的值，不搜索 JSON 字段名

未指定批次且尚未审批的 pending 出库明细没有可反查批次，因此只能命中出库单主表、明细库位和关联物品字段。

不搜索数据库主键、外键、创建/更新时间、数量和成本金额。

### 查询策略

出库搜索应使用 `EXISTS` 子查询，避免一张出库单因多条明细或多条扣减流水在分页结果中重复。

如果 `search` 存在，计数查询和数据查询必须使用同一套过滤条件，保证 `total` 与当前页一致。

## `GET /api/outbound/filter-values`

### 业务语义

返回出库历史列表筛选项。接口不接收参数，不要求前端传模板 ID 或字段名。

这个接口是出库历史视角，按出库单去重计数。批次号、有效期和模板字段值只来自可追溯批次：明细显式指定的 `batch_id`，或审批后 `stock_movements` 写入的扣减批次。

### 字段范围

内置字段首版建议返回：

- `base:destination`：出库去向
- `base:status`：出库单状态
- `base:item`：物品名称
- `base:sku`：物品 SKU
- `base:location`：出库库位
- `base:batch_no`：扣减批次号

模板字段：

- 只统计 `stock_template_fields.searchable = true` 的字段
- 字段值通过扣减批次反查入库明细的 `ext_attributes_json`
- 不要求模板 ID，跨模板同名字段全局合并

### 计数规则

`count` 表示拥有该字段值的去重出库单数量。

同一出库单的多条明细或多条扣减流水都含有同一个值时，只计 1 次。同一出库单确实含有多个不同值时，可以分别计入多个值。

## 分层实现建议

### 路由层

在 `core/src/stock/mod.rs` 增加：

```text
GET /api/items/filter-values
GET /api/inbound/filter-values
GET /api/outbound/filter-values
```

这些接口按资源挂载权限：物品接口使用 `stock.item.read`，入库接口使用 `stock.inbound.read`，出库接口使用 `stock.outbound.read`。

### Controller

建议在现有子模块内扩展：

- `core/src/stock/controller/items.rs`
  - 增加 `item_filter_values` handler
  - 增加筛选值响应 DTO 或复用公共 DTO
- `core/src/stock/controller/inbound.rs`
  - `InboundListQuery` 增加 `search: Option<String>`
  - 增加 `inbound_filter_values` handler
- `core/src/stock/controller/outbound.rs`
  - `OutboundListQuery` 增加 `search: Option<String>`
  - 增加 `outbound_filter_values` handler

若 DTO 在物品、入库和出库间完全共用，可放入 `controller/common.rs`，避免重复定义。

### Service

建议在现有服务子模块内扩展：

- `core/src/stock/service/items.rs`
  - 归一化物品 `search`
  - 调用物品筛选值查询
- `core/src/stock/service/inbound.rs`
  - 归一化入库 `search`
  - 调用入库筛选值查询
- `core/src/stock/service/outbound.rs`
  - 归一化出库 `search`
  - 调用出库筛选值查询

筛选值响应组装可以放到 `service/response.rs`，保持 controller 不处理数据库记录转换。

### Repository

建议在 `StockRepository` 中增加业务语义方法：

```text
list_item_filter_values()
list_inbound_filter_values()
list_outbound_filter_values()
```

并扩展现有列表查询输入：

```text
ListInboundOrders.search: Option<String>
ListOutboundOrders.search: Option<String>
```

物品列表已有 `ListStockItems.search`，只需要扩展 SQL 搜索范围。

SQL 细节建议拆成私有 helper，避免 `stock_repo.rs` 继续膨胀：

```text
item_search_clause(...)
inbound_search_clause(...)
outbound_search_clause(...)
json_scalar_values_clause(...)
```

如果实现时发现 `stock_repo.rs` 继续过长，应优先拆出库存查询相关子模块，而不是把所有 SQL 继续堆在同一个文件。

## JSON 值处理规则

`ext_attributes_json` 只读取 object 的第一层键值。当前模板字段也是一层字段定义，不需要递归搜索。

可搜索和可返回的 JSON 标量：

- string
- integer
- real
- boolean

忽略：

- null
- object
- array
- trim 后为空的字符串

布尔值返回为 `true` 或 `false`。数字按 SQLite 取出的文本形式返回，后续如果前端需要数值范围筛选，再单独设计范围筛选 API。

## 与 `searchable` 的关系

自由文本搜索用于“找得到”，因此搜索 `ext_attributes_json` 时读取所有标量值，不受 `stock_template_fields.searchable` 限制。

筛选值接口用于生成筛选面板，必须控制噪音，因此只返回 `searchable = true` 的模板字段值。

内置字段不受 `searchable` 影响，由接口语义固定定义。

## 不做的事

本方案不做这些内容：

- 不新增数据库表或字段。
- 不引入全文搜索表、外部搜索引擎或额外依赖。
- 不让 API 要求 `template_id` 或 `field_name` 参数。
- 不把 `stock_template_fields` 的字段定义当成已填写值返回。
- 不搜索数据库主键、外键和技术时间字段。
- 不在 Axum 中处理前端筛选面板展示。

如果后续为了性能需要新增索引、生成列、FTS 表或独立搜索表，需要先单独确认数据库迁移策略。

## 测试建议

至少覆盖以下场景：

- `/api/items?search=` 能通过物品名称、SKU、模板名称和当前库存模板值命中物品。
- `/api/items?search=` 不因同一物品多批次命中而重复返回。
- `/api/items?search=` 不通过已经出完的历史批次模板值命中物品。
- `/api/items/filter-values` 只统计 `remaining_quantity > 0` 的当前库存字段值。
- `/api/items/filter-values` 同一物品同一值多批次只计 1 次。
- `/api/inbound?search=` 能通过来源、备注、物品名、批次号、库位和模板值命中入库单。
- `/api/inbound?search=` 一张入库单多条明细命中时只返回一次。
- `/api/inbound/filter-values` 不受当前库存余额影响，已出完批次对应的历史值仍可出现。
- `/api/outbound/filter-values` 统计出库历史筛选值，批次和模板值从指定批次或已审批扣减流水反查。
- 六个接口按资源分别需要 `stock.item.read`、`stock.inbound.read` 或 `stock.outbound.read` 权限。
- 空搜索词仍返回 `400 invalid_request`，保持现有参数校验风格。

实现后运行：

```text
cargo +stable fmt --all -- --check
cargo +stable test -p winestock-core stock_items
cargo +stable test -p winestock-core stock_inbound
cargo +stable test -p winestock-core stock
cargo +stable check --workspace --all-targets
git diff --check
```
