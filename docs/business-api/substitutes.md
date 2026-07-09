# 替代料 API

管理物品之间的替代关系，当主料缺货或停产时快速查找可用替代品。

当前实现状态：已实现 `POST /api/items/{id}/substitutes`、`GET /api/items/substitutes`、`GET /api/items/{id}/substitutes` 和 `DELETE /api/items/{id}/substitutes/{substitute_id}`，并纳入 OpenAPI。绑定接口采用整体替换语义：请求体中的列表会成为该物品替代料关系的最新完整列表。

## 所需权限


- `stock.substitute.read` — 查看替代关系
- `stock.substitute.manage` — 绑定/解绑替代关系

### `POST /api/items/{id}/substitutes`


为指定物品绑定替代品列表。该接口会整体替换当前物品已有替代料关系，并写入 `linked` 审计事件；空列表会清空当前物品所有替代料关系。

- 权限：`stock.substitute.manage`

**请求体：`SubstituteBindRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `substitutes` | array | 是 | 替代品列表 |

**替代品条目：`SubstituteItem`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `substitute_item_id` | integer | 是 | 替代品物品 ID |
| `priority` | integer | 是 | 优先级（1=首选，2=次选，以此类推） |
| `notes` | string | 否 | 兼容性备注 |

- 错误：`400` 自引用、重复替代品、重复优先级或循环绑定（A→B→A）检测到 / `404` 物品不存在

### `GET /api/items/substitutes`

查看全部物品替代关系，用于全局替代料关系列表。

- 权限：`stock.substitute.read`
- 响应：`200` + `Vec<SubstituteRelationResponse>`（含主物品 ID、名称、SKU，替代品 ID、名称、SKU，替代品当前库存量、优先级、备注和创建时间）

### `GET /api/items/{id}/substitutes`


查看物品的替代品列表。

- 权限：`stock.substitute.read`
- 响应：`200` + `Vec<SubstituteDetailResponse>`（含替代品的名称、库存量、优先级、备注和创建时间）

### `DELETE /api/items/{id}/substitutes/{substitute_id}`


解绑单个替代关系。

- 权限：`stock.substitute.manage`
- 响应：`204 No Content`
- 错误：`404` 物品或替代料关系不存在
