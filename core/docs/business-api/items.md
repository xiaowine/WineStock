# 库存物品 API

物品是库存流转的最小业务对象。物品分类、可选属性模板和实际物品属性分别保存，模板不会限制自定义字段。

## 权限

- `stock.item.read`：目录、轻量选择、编辑资料、库存详情、批次和筛选值。
- `stock.item.manage`：创建、更新和软删除。

## 数据结构

创建和更新继续使用 `ItemCreateRequest`、`ItemUpdateRequest`，成功只返回 `ItemMutationResponse { id, updated_at }`。读取契约按目录、选择器、编辑器、库存和批次拆分，不提供万能 `ItemResponse`。

物品资料核心字段：

| 字段 | 说明 |
|---|---|
| `name` | 物品名称 |
| `sku` | 未软删除物品内唯一 SKU |
| `category_id` | 可选物品分类 ID |
| `attribute_template_id` | 可选物品属性模板 ID |
| `image_file_id` | 必选物品主图文件 ID；创建前先上传图片 |
| `image_url` | 响应中的受控主图读取地址 |
| `unit` | 计量单位 |
| `description` | 可选说明 |
| `default_price` | 非负参考单价 |
| `reorder_point` | 非负再订货点 |
| `attributes` | 类型化物品固有属性数组 |

单条属性包含 `definition_id`、完整定义投影、类型化 `value` 和可选 `unit`。模板属性与自定义属性共用定义实体，自定义定义只属于当前物品。

类型规则：数字必须有限；URL 只允许 HTTP/HTTPS；日期使用有效 `YYYY-MM-DD`；布尔值必须是 JSON boolean；模板 select 值必须属于候选项；file 值必须是 `{ "file_id": id }`。

数字属性单位由定义显式控制：`none` 不保存单位，`fixed` 由服务端写入定义中的指定值，`select` 只接受定义候选单位。客户端不能覆盖指定单位。

## 接口

- `POST /api/items`：创建物品、主图、属性和文件绑定，全部处于同一数据库事务；返回轻量命令结果。
- `GET /api/items`：返回实时库存目录，支持分页、搜索、分类、属性模板、库存状态筛选和服务端排序，并返回五项状态计数。
- `GET /api/items/options`：返回入库等选择器使用的轻量物品资料，不返回参考价格、完整属性或库存聚合。
- `GET /api/items/filter-values`：只对当前仍有库存的物品聚合基础字段和模板中标记为 searchable 的物品属性。
- `GET /api/items/{id}`：只返回编辑器恢复草稿需要的基础资料和全部固有属性。
- `GET /api/items/{id}/inventory`：返回实时库存量、价值、补货状态、有效批次数和库位聚合。
- `GET /api/items/{id}/batches`：分页返回仍有余额的批次，默认每页 20 条。
- `PUT /api/items/{id}`：更新基础资料；传入 `attributes` 时整体替换实际属性，成功返回轻量命令结果。
- `DELETE /api/items/{id}`：软删除物品。

更新请求中，字段缺失表示保留原值；`category_id`、`attribute_template_id`、`description`、`default_price` 和 `reorder_point` 明确传 `null` 时会清空。`image_file_id` 不允许清空；更换时必须传当前用户拥有的未绑定图片。原主图在事务成功后解除占用，可通过临时文件删除接口删除或等待孤儿清理。

自由搜索匹配物品基础字段、分类元数据、物品属性模板元数据和实际物品属性。物品属性属于物品自身，因此即使库存已经耗尽，仍可通过属性搜索到该物品；筛选值接口仍按当前库存范围统计。

目录库存状态固定为：零库存 `out_of_stock`；库存大于零且小于等于补货点 `reorder_due`；库存大于零但未设置补货点 `needs_configuration`；其它为 `normal`。`needs_attention` 是缺货和待补货的并集。状态计数受搜索、分类和模板条件影响，但忽略当前库存状态筛选。
