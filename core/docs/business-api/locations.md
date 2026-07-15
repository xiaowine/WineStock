# 库位管理 API

库位是库存批次和出入库流转的属性，不属于物品主数据。库位分组支持父子层级，根分组计为第 1 层，最多 10 层；具体库位归属于某个分组。

当前实现状态：已实现库位分组树、库位 CRUD 和整批次移库接口，并纳入 OpenAPI。本地服务启动时如果没有任何有效库位，会自动创建 `默认库区` 和 `默认库位`。

## 所需权限

- `stock.location.read` — 查看库位分组树和库位列表
- `stock.location.manage` — 管理库位分组、库位和整批次移库

## 库位分组

### `GET /api/location-groups/tree`

查询未删除库位分组树。每个分组节点包含直接子分组和直接库位。

- 权限：`stock.location.read`
- 响应：`200` + `LocationGroupTreeNode[]`

### `POST /api/location-groups`

创建库位分组。

**请求体：`LocationGroupCreateRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `parent_id` | integer/null | 否 | 上级分组 ID；为空表示根分组 |
| `name` | string | 是 | 分组名称，同一上级分组内唯一 |
| `sort_order` | integer | 否 | 排序值，默认 0 |

- 权限：`stock.location.manage`
- 响应：`201` + `LocationGroupResponse`
- 错误：`400 location_group_depth_exceeded` 新分组会超过 10 层

### `PUT /api/location-groups/{id}`

更新库位分组。提交 `parent_id` 可移动分组；为空表示移动到根分组。

- 权限：`stock.location.manage`
- 错误：`400 location_group_cycle` 移动后会形成循环层级 / `400 location_group_depth_exceeded` 移动后的整个子树会超过 10 层 / `409 location_group_name_taken`

### `DELETE /api/location-groups/{id}`

软删除库位分组。

- 权限：`stock.location.manage`
- 规则：仍有子分组或有效库位时返回 `409 location_group_in_use`

## 库位

### `GET /api/locations`

查询未删除库位列表。

- 权限：`stock.location.read`
- 查询参数：`group_id`、`search`
- 响应：`200` + `LocationResponse[]`

### `POST /api/locations`

创建库位。

**请求体：`LocationCreateRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `group_id` | integer | 是 | 所属库位分组 ID |
| `name` | string | 是 | 库位名称，未删除库位内全局唯一 |
| `notes` | string/null | 否 | 库位备注，最多 1024 个字符 |
| `sort_order` | integer | 否 | 排序值，默认 0 |

- 权限：`stock.location.manage`
- 响应：`201` + `LocationResponse`

### `PUT /api/locations/{id}`

更新库位基础资料。

- 权限：`stock.location.manage`
- 错误：`409 location_name_taken`

### `DELETE /api/locations/{id}`

软删除库位。

- 权限：`stock.location.manage`
- 规则：仍有当前库存批次引用时返回 `409 location_in_use`；历史单据引用不阻止软删除。

## 整批次移库

### `POST /api/location-transfers`

把一个仍有余额的批次整体移动到另一个库位。首版不做部分数量移库。

**请求体：`LocationTransferCreateRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `batch_id` | integer | 是 | 需要移动的库存批次 ID |
| `from_location_id` | integer | 是 | 调用方确认的当前原库位 ID |
| `to_location_id` | integer | 是 | 目标库位 ID |
| `notes` | string | 否 | 移库备注 |

- 权限：`stock.location.manage`
- 响应：`201` + `LocationTransferResponse`
- 错误：`404 stock_batch_not_found` / `404 location_not_found` / `400 invalid_request`
