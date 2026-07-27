# 库存物品 API

物品是库存流转的最小业务对象。物品分类、可选属性模板和实际物品属性分别保存，模板不会限制自定义字段。

## 权限

- `stock.item.read`：目录、轻量选择、编辑资料、库存详情、批次和筛选值。
- `stock.item.manage`：创建、更新和软删除。

## 数据结构

创建和更新继续使用 `ItemCreateRequest`、`ItemUpdateRequest`，成功只返回 `ItemMutationResponse { id, updated_at }`。读取契约按目录、选择器、编辑器、库存和批次拆分，不提供万能 `ItemResponse`。

物品资料核心字段：

| 字段                    | 说明                                  |
| ----------------------- | ------------------------------------- |
| `name`                  | 物品名称                              |
| `sku`                   | 未软删除物品内唯一 SKU                |
| `category_id`           | 可选物品分类 ID                       |
| `attribute_template_id` | 可选物品属性模板 ID                   |
| `image_file_id`         | 必选物品主图文件 ID；创建前先上传图片 |
| `image_url`             | 响应中的受控主图读取地址              |
| `unit`                  | 计量单位                              |
| `description`           | 可选说明                              |
| `default_price`         | 非负参考单价                          |
| `reorder_point`         | 非负再订货点                          |
| `attributes`            | 类型化物品固有属性数组                |

单条属性包含 `definition_id`、完整定义投影、类型化 `value` 和可选 `unit`。模板属性与自定义属性共用定义实体，自定义定义只属于当前物品。

类型规则：数字必须有限；URL 只允许 HTTP/HTTPS；日期使用有效 `YYYY-MM-DD`；布尔值必须是 JSON boolean；模板 select 值必须属于候选项；file 值必须是 `{ "file_id": id }`。

数字属性单位由定义显式控制：`none` 不保存单位，`fixed` 由服务端写入定义中的指定值，`select` 只接受定义候选单位。客户端不能覆盖指定单位。

## 接口

- `POST /api/items`：创建物品、主图、属性和文件绑定，全部处于同一数据库事务；返回轻量命令结果。
- `GET /api/items`：返回实时库存目录，支持分页、搜索、分类、属性模板、库存状态、单位、有效库位和可搜索模板属性筛选及服务端排序，并返回五项状态计数。
- `GET /api/items/options`：返回入库等选择器使用的轻量物品资料，不返回参考价格、完整属性或库存聚合。
- `GET /api/items/filter-values`：按当前目录搜索、分类、模板、库存状态和结构化筛选上下文返回分面候选值；模板中只有 `searchable = true` 的共享属性定义参与。
- `GET /api/items/{id}`：只返回编辑器恢复草稿需要的基础资料和全部固有属性。
- `GET /api/items/{id}/inventory`：返回实时库存量、价值、补货状态、有效批次数和库位聚合。
- `GET /api/items/{id}/batches`：分页返回仍有余额的批次，默认每页 20 条。
- `PUT /api/items/{id}`：更新基础资料；传入 `attributes` 时整体替换实际属性，成功返回轻量命令结果。
- `DELETE /api/items/{id}`：软删除物品。
- `GET /api/items/lookups/lcsc/{product_code}`：使用 `stock.item.manage` 权限查询一个 `C` 开头的
  立创商城商品编号，返回归一化候选资料；查询不创建物品、不写文件、库存或审计事件。Core 固定 POST
  `https://so.szlcsc.com/phone/global/query`，请求只包含规范化后的单个 `keyword`、`pageSize = 10`、
  `currentPage = 1`、`searchSource = "main_so"` 和 `asyncRequest = false`。前端不能提供上游 URL、请求头、
  Cookie、token 或批量编号。Core 必须遍历 `result.searchResult.productRecordList`，按
  `productVO.productCode` 与输入编号选择唯一精确匹配项，不能直接采用搜索结果第一项。参考单价取精确匹配
  商品 `productPriceList` 中起订量最小的正数价格；无库存或无有效价格时返回 `null`。

立创查询响应只保留商品编号、候选名称、描述、制造商、制造商型号、封装、数据手册、可选参考单价、过滤后的标量参数
和可空 `image_url`，不公开原始 JSON。图片地址只接受 HTTPS、严格主机 `alimg.szlcsc.com` 和
`/upload/public/product/` 或 `/upload/public/brand/product/certificate/` 路径。格式无效、未找到、并发繁忙、超时、上游失败和
无效响应分别返回稳定错误码 `invalid_lcsc_product_code`、`lcsc_product_not_found`、`lcsc_lookup_busy`、
`lcsc_lookup_timeout`、`lcsc_lookup_failed`、`lcsc_invalid_response`。
商品图只按查询响应字段选择，不在 Core 查询阶段下载或探测图片：读取 `luceneBreviaryImageUrls` 的首项（缺失时使用
`breviaryImageUrl`），能从受控 `/breviary/` 路径生成 `/source/` 地址时优先返回 source；否则返回该首张
breviary，再回退到 `bigImageUrl`，均无受控地址时返回 `null`。
候选名称优先取 `attributes["Manufacturer Part"]`，再回退到 `attributes["LCSC Part Name"]` 和商品编号；
候选描述优先取顶层 `description`，再回退到 `attributes["LCSC Part Name"]`，均不存在时返回 `null`。
前端确认覆盖后直接从白名单图片地址发起无凭据、无自定义头的简单 GET，复核状态、MIME、15 MiB 上限和
PNG/JPEG/WebP 文件签名，再把 Blob 作为普通临时图片走现有 `POST /api/files/images` 上传和 `image_file_id`
创建流程；图片缺失或读取失败时生成带客编的默认占位图，不得阻断其它候选资料和参考单价回填。

更新请求中，字段缺失表示保留原值；`category_id`、`attribute_template_id`、`description`、`default_price` 和 `reorder_point` 明确传 `null` 时会清空。`image_file_id` 不允许清空；更换时必须传当前用户拥有的未绑定图片。原主图在事务成功后解除占用，可通过临时文件删除接口删除或等待孤儿清理。

自由搜索匹配物品基础字段、分类元数据、物品属性模板元数据和实际物品属性。物品属性属于物品自身，因此即使库存已经耗尽，仍可通过属性搜索到该物品。

结构化字段通过可选 `filters` 查询参数传入 URL 编码的 JSON 数组。同一字段多值按 OR，不同字段及搜索、分类、模板、库存状态之间按 AND。基础 key 当前为 `base:unit` 和 `base:location`；模板属性使用 `template:<definition_id>`。JSON 最大 `4096` 字节，最多 `12` 个字段、每字段 `20` 个值、单值最长 `256` 个字符；未知 key、不可搜索定义、非法 JSON 或超限输入返回 `invalid_request`。

筛选值接口接受与目录相同的搜索和筛选上下文。计算某字段候选值时排除该字段自身条件、应用其它条件，因此候选计数可以用于多选分面筛选。库位只由仍有余额的批次贡献；文件类型属性不进入候选值。

目录库存状态固定为：零库存 `out_of_stock`；库存大于零且小于等于补货点 `reorder_due`；库存大于零但未设置补货点 `needs_configuration`；其它为 `normal`。`needs_attention` 是缺货和待补货的并集。状态计数受搜索、分类、模板和结构化字段条件影响，但忽略当前库存状态筛选。
