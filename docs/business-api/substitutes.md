# 替代料 API

管理物品之间的替代关系，当主料缺货或停产时快速查找可用替代品。

当前实现状态：已实现 `GET /api/substitutes`、`GET /api/substitutes/{item_id}`、`PUT /api/substitutes/{item_id}` 和 `DELETE /api/substitutes/{item_id}/{substitute_item_id}`，并纳入 OpenAPI。替换接口采用整体替换语义：请求体中的列表会成为该物品替代料关系的最新完整列表。

## 所需权限

- `stock.substitute.read` — 查看替代关系
- `stock.substitute.manage` — 整体替换或删除替代关系

### `GET /api/substitutes`

查看全部物品替代关系，用于全局替代料关系列表。

- 权限：`stock.substitute.read`
- 响应：`200` + `Vec<SubstituteRelationResponse>`（含主物品 ID、名称、SKU，替代品 ID、名称、SKU，替代品当前库存量、优先级、备注和创建时间）

### `GET /api/substitutes/{item_id}`

查看指定物品的替代品列表。

- 权限：`stock.substitute.read`
- 响应：`200` + `Vec<ItemSubstituteResponse>`（含替代品的名称、库存量、优先级、备注和创建时间）
- 错误：`404` 物品不存在

### `PUT /api/substitutes/{item_id}`

整体替换指定物品的替代品列表。该接口会删除旧替代料关系、写入请求体中的新关系，并写入替代料审计事件；非空列表记录 `linked`，空列表清空当前物品所有替代料关系并记录 `unlinked`，详情包含旧列表、新列表、实际新增和实际移除的物品 ID。

- 权限：`stock.substitute.manage`

**请求体：`SubstituteReplaceRequest`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `substitutes` | array | 是 | 替代品列表；空数组表示清空 |

**替代品条目：`SubstituteReplacementItem`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `substitute_item_id` | integer | 是 | 替代品物品 ID |
| `priority` | integer | 是 | 优先级（1=首选，2=次选，以此类推） |
| `notes` | string | 否 | 兼容性备注 |

- 响应：`200` + `Vec<ItemSubstituteResponse>`
- 错误：`400` 自引用、重复替代品、重复优先级或循环绑定（A→B→A）检测到 / `404` 物品不存在

### `DELETE /api/substitutes/{item_id}/{substitute_item_id}`

删除单个替代关系。

- 权限：`stock.substitute.manage`
- 响应：`204 No Content`
- 错误：`404` 物品或替代料关系不存在
