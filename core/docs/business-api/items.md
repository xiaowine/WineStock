# 库存物品 API

物品是库存流转的最小业务对象。物品分类、可选属性模板和实际物品属性分别保存，模板不会限制自定义字段。

## 权限

- `stock.item.read`：列表、详情和筛选值。
- `stock.item.manage`：创建、更新和软删除。

## 数据结构

`ItemCreateRequest`、`ItemUpdateRequest` 和 `ItemResponse` 的核心字段：

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

单条属性包含可选 `template_field_id`、`field_name`、`field_type`、类型化 `value` 和可选 `unit`。字段名在同一物品内唯一。自定义属性的 `template_field_id` 为空。

类型规则：数字必须有限；URL 只允许 HTTP/HTTPS；日期使用有效 `YYYY-MM-DD`；布尔值必须是 JSON boolean；模板 select 值必须属于候选项；file 值必须是 `{ "file_id": id }`。

模板属性的单位由对应模板字段显式控制：`none` 不保存单位，`fixed` 由服务端写入模板固定值，`select` 只接受模板候选单位，`custom` 才接受自由单位。客户端不能通过提交其它单位覆盖固定规则。自定义属性仍可携带可选自由单位。

## 接口

- `POST /api/items`：创建物品、主图、属性和文件绑定，全部处于同一数据库事务。主图必须是当前用户拥有的未绑定 PNG、JPEG 或 WebP。
- `GET /api/items`：按页返回基础资料和实际物品属性；支持 `page`、`page_size`、`category_id` 和非空 `search`。
- `GET /api/items/filter-values`：只对当前仍有库存的物品聚合基础字段和模板中标记为 searchable 的物品属性。
- `GET /api/items/{id}`：返回基础资料、物品属性、当前数量、价值、库位分布和批次摘要。
- `PUT /api/items/{id}`：更新基础资料；传入 `attributes` 时整体替换实际属性。
- `DELETE /api/items/{id}`：软删除物品。

更新请求中，字段缺失表示保留原值；`category_id`、`attribute_template_id`、`description`、`default_price` 和 `reorder_point` 明确传 `null` 时会清空。`image_file_id` 不允许清空；更换时必须传当前用户拥有的未绑定图片。原主图在事务成功后解除占用，可通过临时文件删除接口删除或等待孤儿清理。

自由搜索匹配物品基础字段、分类元数据、物品属性模板元数据和实际物品属性。物品属性属于物品自身，因此即使库存已经耗尽，仍可通过属性搜索到该物品；筛选值接口仍按当前库存范围统计。
