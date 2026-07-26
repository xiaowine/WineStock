# 入库 API

入库明细只记录数量、价格、库位、批次和有效期。物品型号、品牌、参数、材质和产品图片属于物品属性，不由入库单保存。

## `POST /api/inbound`

按 `submission_mode` 创建待审批单据或直接完成入库。两种模式都要求 `stock.inbound.create`；`direct` 还要求 `stock.inbound.approve`。

请求主字段：`submission_mode`、`source`、可选 `notes` 和至少一条 `items`。

| `submission_mode` | 结果 |
|---|---|
| `pending_approval` | 创建 `pending` 单据，不增加库存，等待后续审批 |
| `direct` | 在创建事务内完成审批、批次、库存流水和审计写入，返回 `approved` |

创建响应顶层返回实际采用的 `submission_mode`，前端据此区分“已提交审核”和“已直接入库”，不通过权限或审批字段反推。

每条明细包含：

| 字段 | 说明 |
|---|---|
| `item_id` | 有效物品 ID |
| `quantity` | 大于 0 的数量 |
| `unit_price` | 非负单价 |
| `location_id` | 有效库位 ID |
| `batch_no` | 可选外部批次号 |
| `expires_at` | 可选有效期 |

同一物品可以在一个请求中出现多次，每条明细独立保存数量、价格、库位、批次和有效期。

入库单和明细在同一事务中提交；直接入库时，批次和库存流水也属于同一个事务。

结构化错误包含零基 `line_index`。
没有 `stock.inbound.approve` 的用户请求 `direct` 时返回 `403 inbound_direct_approval_forbidden`。

## 查询与审批

- `GET /api/inbound`：权限 `stock.inbound.read`，支持分页、物品、状态（`pending`、`approved`、`rejected`）、日期和自由搜索；状态筛选在服务端与分页总数共同生效。
- `GET /api/inbound/filter-values`：按全部入库历史聚合来源、状态、物品、库位和批次号等基础字段。
- `GET /api/inbound/{id}`：返回单据、明细及关联物品的名称、编码、单位和主图文件 ID。
- `POST /api/stock-approvals/inbound/{id}/approve`：权限 `stock.inbound.approve`；重新确认明细物品仍有效后生成批次与库存流水。
- `POST /api/stock-approvals/inbound/{id}/reject`：权限 `stock.inbound.approve`；只更新单据状态。

## 图片文件

### `POST /api/files/images`

权限为 `stock.item.manage`。multipart 单文件上传，支持 PNG、JPEG、WebP，最大 15MB，同时校验声明 MIME、真实签名和大小。返回文件 ID、名称、MIME、大小和受控读取地址；同一接口用于物品必选主图和扩展图片属性。

### `GET /api/files/{id}`

- 未绑定文件仅上传所有者可读。
- 绑定物品属性后要求 `stock.item.read` 或 `stock.item.manage`。

### `DELETE /api/files/{id}`

仅所有者可删除尚未绑定的文件。物品绑定存在时返回 `409 file_already_bound`。服务启动和每次上传前会清理超过 24 小时仍未绑定的文件元数据与无引用磁盘内容，也会回收写盘后、元数据创建前中断留下的无记录文件。
