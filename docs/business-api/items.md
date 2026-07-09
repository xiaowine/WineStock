# 库存物品 API

基础库存物品实体 CRUD。物品是库存流转的最小单位。

当前实现状态：已实现 `POST /api/items`、`GET /api/items`、`GET /api/items/filter-values`、`GET /api/items/{id}`、`PUT /api/items/{id}` 和 `DELETE /api/items/{id}`，并纳入 OpenAPI。

## 所需权限


- `stock.item.read` — 查看物品列表、详情和物品筛选值
- `stock.item.manage` — 创建、修改、删除物品

## 数据结构


`ItemCreateRequest` / `ItemUpdateRequest` / `ItemResponse`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 物品名称 |
| `sku` | string | 是 | 物品编号/SKU，唯一 |
| `category_id` | integer | 否 | 所属分类 ID，关联到分类模板 |
| `unit` | string | 是 | 计量单位（个/米/KG/件等） |
| `description` | string | 否 | 描述 |
| `default_price` | number | 否 | 参考单价 |
| `reorder_point` | number | 否 | 再订货点，库存低于此值时提醒 |

## 接口列表

### `POST /api/items`


创建新物品。

- 权限：`stock.item.manage`
- 请求：`ItemCreateRequest`
- 响应：`201` + `ItemResponse`
- 错误：`400` 参数校验失败 / `409` SKU 重复

### `GET /api/items`


分页查询物品列表。

- 权限：`stock.item.read`
- 查询参数：`page`、`page_size`、`category_id`（按分类筛选）、`search`（可选；不传时返回列表，传入非空值时按物品基础字段、模板元数据和当前库存模板值模糊搜索）
- 响应：`200` + `PaginatedResponse<ItemResponse>`
- 说明：模板实际值只从 `stock_batches.remaining_quantity > 0` 的当前库存批次追溯；同一物品多批次命中时结果仍按物品去重。空 `search` 返回 `400 invalid_request`。

### `GET /api/items/filter-values`

查询物品列表筛选值。

- 权限：`stock.item.read`
- 查询参数：无
- 响应：`200` + `FilterValuesResponse`
- 统计范围：当前库存视角，只统计 `remaining_quantity > 0` 的批次。
- 首版内置字段：`base:category`、`base:unit`、`base:location`
- 模板字段：只返回 `stock_template_fields.searchable = true` 的一层 JSON 标量值；同名字段跨模板合并。
- 计数：`count` 表示拥有该字段值的去重物品数量。

### `GET /api/items/{id}`


查看单个物品详情，包含物品基础资料、当前库存总量、库存价值、库位分布和当前有效批次摘要。物品主数据不保存库位，库位分布来自当前有效批次的 `location_id`。

- 权限：`stock.item.read`
- 响应：`200` + `ItemDetailResponse`
  - `current_quantity`：当前剩余库存总量，只统计 `stock_batches.remaining_quantity > 0` 的批次
  - `inventory_value`：当前库存价值，按批次剩余数量乘以批次单价汇总
  - `locations`：当前库存按库位聚合的数量、价值和批次数，包含 `location_id`、`location_code` 和 `location_name`
  - `batches`：当前仍有余额的批次摘要，包含批次号、库位 ID/编码/名称、初始数量、剩余数量、单价、价值、入库时间和有效期
- 错误：`404` 物品不存在

### `PUT /api/items/{id}`


更新物品信息。

- 权限：`stock.item.manage`
- 请求：`ItemUpdateRequest`（所有字段可选，只提交修改的部分）
- 响应：`200` + `ItemResponse`
- 错误：`404` / `409` SKU 冲突

### `DELETE /api/items/{id}`


删除物品（软删除）。

- 权限：`stock.item.manage`
- 响应：`204 No Content`
- 错误：`404`
