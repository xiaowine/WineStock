# 出库 API

### `POST /api/outbound`

创建 `pending` 出库单。创建阶段只保存单据和明细，不扣减库存；审批通过后才按指定批次或 FIFO 扣减库存。

- 权限：`stock.outbound.create`

**请求体：`OutboundCreateRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `destination` | string | 是 | 去向（项目名称 / 部门 / 客户） |
| `items` | array | 是 | 出库物品明细 |
| `notes` | string | 否 | 备注 |

**出库明细条目：`OutboundItem`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `item_id` | integer | 是 | 物品 ID |
| `quantity` | number | 是 | 出库数量 |
| `batch_id` | integer | 否 | 指定消耗批次 ID；为空时按 FIFO 自动选择 |
| `location_id` | integer | 否 | 指定库位；为空时按全部当前库存 FIFO 扣减 |

- 响应：`201` + `OutboundResponse`，状态为 `pending`
- 响应明细会返回 `location_id` 和 `location_name`；未指定库位时这些字段为空。
- 错误：`400` 请求无效 / `404` 物品 ID 不存在

### `GET /api/outbound`


分页查询出库单列表。

- 权限：`stock.outbound.read`
- 查询参数：`page`、`page_size`、`item_id`、`status`（`pending`、`approved`、`rejected`）、`date_from`、`date_to`、`search`（可选；不传时返回列表，传入非空值时搜索）
- 响应：`200` + `PaginatedResponse<OutboundResponse>`
- 说明：出库单搜索会匹配出库去向、备注、状态、库位名称、关联物品基础字段；对已指定批次或已审批写入流水的明细，还会匹配批次号、有效期和入库模板实际值。结果按出库单去重。空 `search` 返回 `400 invalid_request`；状态筛选同时作用于列表与总数。每条响应明细投影关联物品的名称、编码、单位和主图文件 ID，客户端不得逐行补请求。

### `GET /api/outbound/filter-values`

查询出库历史筛选值。

- 权限：`stock.outbound.read`
- 查询参数：无
- 响应：`200` + `FilterValuesResponse`
- 统计范围：出库历史视角；批次和模板值从指定批次或已审批扣减流水反查。
- 首版内置字段：`base:destination`、`base:status`、`base:item`、`base:sku`、`base:location`、`base:batch_no`；`base:location` 的值为全局唯一库位名称。
- 入库属性字段：只返回 `stock_inbound_template_fields.searchable = true` 且通过实际出库批次追溯到的标量值；同名字段跨模板合并。
- 计数：`count` 表示拥有该字段值的去重出库单数量。

### `GET /api/outbound/{id}`


查看出库单详情。

- 权限：`stock.outbound.read`
- 响应：`200` + `OutboundResponse`
