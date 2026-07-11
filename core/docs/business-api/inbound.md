# 入库 API

入库核心流程：选择物品（自带分类）→ 选择库位 → 按该分类模板填写 `ext_attributes` → 服务端校验后生成入库单和批次。

### `POST /api/inbound`

创建 `pending` 入库单，同时携带模板化扩展属性。创建阶段只保存单据和明细，不生成批次、不写库存流水。

- 权限：`stock.inbound.create`

**请求体：`InboundCreateRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `source` | string | 是 | 来源（供应商名称或采购单号 PO） |
| `items` | array | 是 | 入库物品明细 |
| `notes` | string | 否 | 备注 |

**入库明细条目：`InboundItem`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `item_id` | integer | 是 | 物品 ID |
| `quantity` | number | 是 | 入库数量 |
| `unit_price` | number | 是 | 采购单价 |
| `location_id` | integer | 是 | 存储库位 ID，必须指向未删除库位 |
| `batch_no` | string | 否 | 外部批次号（为空时服务端自动生成） |
| `expires_at` | string (date) | 否 | 有效期（若适用） |
| `ext_attributes` | object | 否 | 模板化扩展属性，按物品分类模板校验（如电子元件填 `{"封装":"0603","品牌":"ST"}`） |

- 响应：`201` + `InboundResponse`，状态为 `pending`
- 响应明细会返回 `location_id`、`location_code` 和 `location_name`。
- 错误：`400` `ext_attributes` 不满足模板约束 / `404` 物品 ID 不存在

### `GET /api/inbound`


分页查询入库单列表。

- 权限：`stock.inbound.read`
- 查询参数：`page`、`page_size`、`item_id`、`date_from`、`date_to`、`search`（可选；不传时返回列表，传入非空值时搜索）
- 响应：`200` + `PaginatedResponse<InboundResponse>`
- 说明：入库单搜索会匹配入库来源、备注、状态、库位编码/名称、批次号、有效期、关联物品基础字段和入库模板实际值；结果按入库单去重。空 `search` 返回 `400 invalid_request`。

### `GET /api/inbound/filter-values`

查询入库历史筛选值。

- 权限：`stock.inbound.read`
- 查询参数：无
- 响应：`200` + `FilterValuesResponse`
- 统计范围：入库历史视角，不受当前库存余额影响。
- 首版内置字段：`base:source`、`base:status`、`base:item`、`base:sku`、`base:location`、`base:batch_no`；`base:location` 的值为库位编码。
- 模板字段：只返回 `stock_template_fields.searchable = true` 的一层 JSON 标量值；同名字段跨模板合并。
- 计数：`count` 表示拥有该字段值的去重入库单数量。

### `GET /api/inbound/{id}`


查看入库单详情（含入库明细和扩展属性）。

- 权限：`stock.inbound.read`
- 响应：`200` + `InboundResponse`
