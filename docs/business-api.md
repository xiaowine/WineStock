 # WineStock 业务 API 文档
 
本文档记录 WineStock 当前业务 REST API 设计和实现状态，作为库存、出入库、看板、替代料与审计能力的长期接口文档。
 每个接口的设计风格与当前 Core 中 auth/users 模块一致：
 
 - 统一使用 `POST/GET/PUT/DELETE` + JSON body（`ValidatedJson` + `#[serde(deny_unknown_fields)]`）
 - 需要鉴权的接口统一使用 bearer token（`CurrentUser` extractor + `AuthorizeRouteExt`）
 - 返回错误统一使用 `AuthApiError`（或后续新增 `*/BusinessApiError`）映射到标准 HTTP 状态码
 - DTO 使用 `garde::Validate` + `utoipa::ToSchema`，以支持请求校验和 OpenAPI 输出
 
当前 RBAC 启动会补齐本文档列出的库存和审计权限代码。业务授权统一通过 route layer 的 `AuthorizeRouteExt` 判断权限代码。
 
 ---
 
 ## 1. 库存物品管理（Core Stock / Items）
 
基础库存物品实体 CRUD。物品是库存流转的最小单位。

当前实现状态：已实现 `POST /api/items`、`GET /api/items`、`GET /api/items/filter-values`、`GET /api/items/{id}`、`PUT /api/items/{id}` 和 `DELETE /api/items/{id}`，并纳入 OpenAPI。
 
 ### 所需新增权限
 
 - `stock.item.manage` — 创建、修改、删除物品
 - `stock.read` — 查看物品列表和详情
 
 ### 数据结构
 
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
 
 ### 接口列表
 
 #### `POST /api/items`
 
 创建新物品。
 
 - 权限：`stock.item.manage`
 - 请求：`ItemCreateRequest`
 - 响应：`201` + `ItemResponse`
 - 错误：`400` 参数校验失败 / `409` SKU 重复
 
 #### `GET /api/items`
 
 分页查询物品列表。
 
- 权限：`stock.read`
- 查询参数：`page`、`page_size`、`search`（按物品基础字段、模板元数据和当前库存模板值模糊搜索）、`category_id`（按分类筛选）
- 响应：`200` + `PaginatedResponse<ItemResponse>`
- 说明：模板实际值只从 `stock_batches.remaining_quantity > 0` 的当前库存批次追溯；同一物品多批次命中时结果仍按物品去重。空 `search` 返回 `400 invalid_request`。

#### `GET /api/items/filter-values`

查询物品列表筛选值。

- 权限：`stock.read`
- 查询参数：无
- 响应：`200` + `FilterValuesResponse`
- 统计范围：当前库存视角，只统计 `remaining_quantity > 0` 的批次。
- 首版内置字段：`base:category`、`base:unit`、`base:location`
- 模板字段：只返回 `stock_template_fields.searchable = true` 的一层 JSON 标量值；同名字段跨模板合并。
- 计数：`count` 表示拥有该字段值的去重物品数量。
 
#### `GET /api/items/{id}`
 
 查看单个物品详情。
 
 - 权限：`stock.read`
- 响应：`200` + `ItemResponse`（首版尚未包含当前库存、库位分布和批次摘要）
 - 错误：`404` 物品不存在
 
 #### `PUT /api/items/{id}`
 
 更新物品信息。
 
 - 权限：`stock.item.manage`
 - 请求：`ItemUpdateRequest`（所有字段可选，只提交修改的部分）
 - 响应：`200` + `ItemResponse`
 - 错误：`404` / `409` SKU 冲突
 
 #### `DELETE /api/items/{id}`
 
删除物品（软删除）。
 
 - 权限：`stock.item.manage`
 - 响应：`204 No Content`
- 错误：`404`
 
 ---
 
## 2. 入库 / 出库管理（Inbound / Outbound）

管理物品的物理入库和出库操作，核心在于批次跟踪和库存事务一致性。

### 所需新增权限

- `stock.inbound.create` — 创建入库单
- `stock.outbound.create` — 创建出库单
- `stock.inbound.approve` / `stock.outbound.approve` — 审批入库/出库单（可选，与 RBAC 状态机扩展相关）
 - `stock.template.manage` — 管理入库模板定义

 ### 2.1 入库模板管理（入库配置前置）
 
 入库天然是模板化的：物品分类决定了入库时需填写的扩展字段。
 模板管理是入库的配置前置，不属于独立业务领域。

当前实现状态：已实现 `POST /api/templates`、`GET /api/templates`、`GET /api/templates/{id}`、`PUT /api/templates/{id}`、`DELETE /api/templates/{id}` 和 `POST /api/templates/{id}/copy`，并纳入 OpenAPI。

本地服务启动后会补齐内置模板：`元器件`、`3D打印耗材` 和 `通用`。补齐只在不存在同名模板记录时创建，不覆盖用户修改，也不恢复用户已经软删除的同名模板。
 
 #### `POST /api/templates`
 
 创建新的分类模板。
 
 - 权限：`stock.template.manage`
 
 | 字段 | 类型 | 必填 | 说明 |
 |------|------|------|------|
 | `name` | string | 是 | 模板名称 |
 | `description` | string | 否 | 说明 |
 | `fields` | array | 是 | 模板字段定义列表 |
 
 **字段定义：`TemplateFieldDef`**
 
 | 字段 | 类型 | 必填 | 说明 |
 |------|------|------|------|
 | `field_name` | string | 是 | 字段名称 |
 | `field_type` | string (enum) | 是 | 字段类型：`text` / `number` / `select` / `date` / `file` / `url` / `boolean` |
 | `required` | boolean | 否 | 是否必填，默认 false |
 | `searchable` | boolean | 否 | 是否可用于筛选，默认 false |
 | `options` | array[string] | 否 | 当 `field_type` 为 `select` 时，预置可选值 |
 | `default_value` | string | 否 | 默认值 |
 
 - 响应：`201` + `TemplateResponse`
 - 错误：`400` 名称重复或字段定义不合法
 
 #### `GET /api/templates`
 
 模板列表。
 
 - 权限：`stock.read`
 - 响应：`200` + `Vec<TemplateResponse>`
 
 #### `GET /api/templates/{id}`
 
 模板详情，含字段定义。
 
 - 权限：`stock.read`
 
 #### `PUT /api/templates/{id}`
 
 更新模板定义。更新后只会影响新入库单，不会回填已有物品扩展属性。
 
 - 权限：`stock.template.manage`
 
 #### `DELETE /api/templates/{id}`
 
 删除模板（软删除）。
 
 - 权限：`stock.template.manage`
 - 错误：`404` 模板不存在 / `409` 仍有未删除物品关联此模板时拒绝
 
 #### `POST /api/templates/{id}/copy`
 
 复制模板。
 
 - 权限：`stock.template.manage`
 - 请求：`{ "name": string }`（可指定新名称）
 - 响应：`201` + `TemplateResponse`
 
 ### 2.2 入库
 
 入库核心流程：选择物品（自带分类）→ 按该分类模板填写 `ext_attributes` → 服务端校验后生成入库单和批次。
 
#### `POST /api/inbound`

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
 | `location` | string | 否 | 存储库位，如 `A1-2-03` |
 | `batch_no` | string | 否 | 外部批次号（为空时服务端自动生成） |
 | `expires_at` | string (date) | 否 | 有效期（若适用） |
 | `ext_attributes` | object | 否 | 模板化扩展属性，按物品分类模板校验（如电子元件填 `{"封装":"0603","品牌":"ST"}`） |
 
- 响应：`201` + `InboundResponse`，状态为 `pending`
- 错误：`400` `ext_attributes` 不满足模板约束 / `404` 物品 ID 不存在

#### `GET /api/inbound`
 
 分页查询入库单列表。
 
- 权限：`stock.read`
- 查询参数：`page`、`page_size`、`item_id`、`date_from`、`date_to`、`search`
- 响应：`200` + `PaginatedResponse<InboundResponse>`
- 说明：`search` 会匹配入库来源、备注、状态、明细库位、批次号、有效期、关联物品基础字段和入库模板实际值；结果按入库单去重。空 `search` 返回 `400 invalid_request`。

#### `GET /api/inbound/filter-values`

查询入库历史筛选值。

- 权限：`stock.read`
- 查询参数：无
- 响应：`200` + `FilterValuesResponse`
- 统计范围：入库历史视角，不受当前库存余额影响。
- 首版内置字段：`base:source`、`base:status`、`base:item`、`base:sku`、`base:location`、`base:batch_no`
- 模板字段：只返回 `stock_template_fields.searchable = true` 的一层 JSON 标量值；同名字段跨模板合并。
- 计数：`count` 表示拥有该字段值的去重入库单数量。
 
 #### `GET /api/inbound/{id}`
 
 查看入库单详情（含入库明细和扩展属性）。
 
- 权限：`stock.read`
- 响应：`200` + `InboundResponse`

#### `POST /api/inbound/{id}/approve`

审批入库单。服务端只允许审批 `pending` 单据；审批前按物品关联模板校验 `ext_attributes`，审批事务内生成批次、写入库存流水和审计事件。

- 权限：`stock.inbound.approve`
- 响应：`200` + `InboundResponse`，状态为 `approved`
- 错误：`400` 扩展属性不满足模板约束 / `404` 入库单不存在 / `409` 单据不是 `pending`

#### `POST /api/inbound/{id}/reject`

拒绝入库单。拒绝只更新单据状态并写审计事件，不改变库存；被拒绝单据不能再审批。

- 权限：`stock.inbound.approve`
- 响应：`200` + `InboundResponse`，状态为 `rejected`
- 错误：`404` 入库单不存在 / `409` 单据不是 `pending`
 
 ### 2.3 出库
 
#### `POST /api/outbound`

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
 | `location` | string | 否 | 指定库位 |
 
- 响应：`201` + `OutboundResponse`，状态为 `pending`
- 错误：`400` 请求无效 / `404` 物品 ID 不存在

#### `GET /api/outbound`
 
 分页查询出库单列表。
 
 - 权限：`stock.read`
 - 查询参数：`page`、`page_size`、`item_id`、`date_from`、`date_to`
 - 响应：`200` + `PaginatedResponse<OutboundResponse>`
 
 #### `GET /api/outbound/{id}`
 
查看出库单详情。

- 权限：`stock.read`
- 响应：`200` + `OutboundResponse`

#### `POST /api/outbound/{id}/approve`

审批出库单。服务端只允许审批 `pending` 单据；明细指定 `batch_id` 时只扣指定批次，未指定时按 `expires_at ASC NULLS LAST, received_at ASC, id ASC` 的 FIFO 规则扣减。库存不足或指定批次不可用时返回冲突并回滚整个审批事务。

- 权限：`stock.outbound.approve`
- 响应：`200` + `OutboundResponse`，状态为 `approved`
- 错误：`404` 出库单不存在 / `409` 单据不是 `pending` 或库存不足

#### `POST /api/outbound/{id}/reject`

拒绝出库单。拒绝只更新单据状态并写审计事件，不扣减库存；被拒绝单据不能再审批。

- 权限：`stock.outbound.approve`
- 响应：`200` + `OutboundResponse`，状态为 `rejected`
- 错误：`404` 出库单不存在 / `409` 单据不是 `pending`
 
---

## 3. 总览看板（Dashboard）
 
 库存全局统计摘要数据接口，供前端仪表盘消费。

当前实现状态：已实现 `GET /api/dashboard/overview` 和 `GET /api/dashboard/trends`，并纳入 OpenAPI。看板统计只读取当前批次剩余库存和审批后生成的 `stock_movements`，不会把 `pending` 或 `rejected` 单据计入出入库数量。
 
 ### 所需新增权限
 
 - `stock.read`
 
 #### `GET /api/dashboard/overview`
 
 库存总览卡片数据。
 
 - 权限：`stock.read`
 
 **响应：`DashboardOverviewResponse`**
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `total_items` | integer | 库存物品种类数 |
 | `total_quantity` | number | 总件数 |
 | `total_value` | number | 当前批次剩余数量乘以批次成本后的库存总价值 |
 | `inbound_3d` | number | 近 3 天入库总数 |
 | `outbound_3d` | number | 近 3 天出库总数 |
 | `slow_moving_items` | array | 呆滞料列表（当前有库存且 30 天内无出入库流水的物品） |
 
 **呆滞料条目：`SlowMovingItem`**
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `item_id` | integer | 物品 ID |
 | `item_name` | string | 物品名称 |
 | `quantity` | number | 当前库存量 |
 | `value` | number | 库存价值 |
 | `days_since_last_movement` | integer | 最近一次出入库距今的天数 |
 
 #### `GET /api/dashboard/trends`
 
 出入库趋势数据，用于近 30 天或自定义范围的可视化。
 
 - 权限：`stock.read`
 - 查询参数：`days`（默认 30，最大 365；小于 1 时按 1 处理）
 
 **响应：`TrendsResponse`**
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `daily` | array[DailyTrend] | 每日入库/出库统计数据 |
 
 **`DailyTrend`**
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `date` | string (date) | 日期 |
 | `inbound_quantity` | number | 入库数量 |
 | `outbound_quantity` | number | 出库数量 |
 
---

## 4. 替代料管理（Substitute Management）
 
 管理物品之间的替代关系，当主料缺货或停产时快速查找可用替代品。

当前实现状态：已实现 `POST /api/items/{id}/substitutes`、`GET /api/items/{id}/substitutes` 和 `DELETE /api/items/{id}/substitutes/{substitute_id}`，并纳入 OpenAPI。绑定接口采用整体替换语义：请求体中的列表会成为该物品替代料关系的最新完整列表。
 
 ### 所需新增权限
 
 - `stock.substitute.manage` — 绑定/解绑替代关系
 - `stock.read` — 查看替代关系
 
 #### `POST /api/items/{id}/substitutes`
 
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
 
 #### `GET /api/items/{id}/substitutes`
 
 查看物品的替代品列表。
 
 - 权限：`stock.read`
 - 响应：`200` + `Vec<SubstituteDetailResponse>`（含替代品的名称、库存量、优先级、备注和创建时间）
 
 #### `DELETE /api/items/{id}/substitutes/{substitute_id}`
 
 解绑单个替代关系。
 
- 权限：`stock.substitute.manage`
- 响应：`204 No Content`
- 错误：`404` 物品或替代料关系不存在
 
---

## 5. 事件日志（Event Logs）
 
 审计和操作追溯记录。

当前实现状态：已实现 `GET /api/events`，并纳入 OpenAPI。事件日志读取 `audit_events`，支持按实体、动作、用户和时间范围筛选，按 `timestamp DESC, id DESC` 返回。
 
 ### 所需新增权限
 
 - `audit.read` — 查看事件日志
 
 #### `GET /api/events`
 
 分页查询事件日志。
 
 - 权限：`audit.read`
 
 **查询参数：**
 
 | 参数 | 类型 | 说明 |
 |------|------|------|
 | `page` | integer | 页码，默认 1 |
 | `page_size` | integer | 每页条数，默认 50 |
 | `entity_type` | string | 筛选实体类型（item / inbound / outbound / template / user / substitute） |
 | `entity_id` | integer | 筛选实体 ID |
 | `action` | string | 操作类型（created / updated / deleted / approved / rejected / linked / unlinked） |
 | `user_id` | integer | 操作人 |
 | `date_from` | string (datetime) | 起始时间 |
 | `date_to` | string (datetime) | 结束时间 |
 
 **响应：`PaginatedResponse<EventLogResponse>`**
 
 `EventLogResponse`
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `id` | integer | 日志 ID |
 | `timestamp` | string (datetime) | 操作时间 |
 | `user_id` | integer | 操作人 ID |
 | `username` | string/null | 操作人用户名；用户外键为空时返回 null |
 | `entity_type` | string | 实体类型 |
 | `entity_id` | integer | 实体 ID |
 | `action` | string | 操作类型 |
 | `details` | object/null (json) | 变更详情（JSON 格式，记录前后差异或关键摘要） |
 
 ---
 
## 分页响应通用结构
 
 `PaginatedResponse<T>`
 
 | 字段 | 类型 | 说明 |
 |------|------|------|
 | `items` | array[T] | 当前页数据 |
 | `total` | integer | 总记录数 |
 | `page` | integer | 当前页码 |
 | `page_size` | integer | 每页条数 |
| `total_pages` | integer | 总页数 |

## 筛选值响应通用结构

`FilterValuesResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `fields` | array[`FilterFieldResponse`] | 可用于当前列表筛选的字段集合 |

`FilterFieldResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `key` | string | 稳定筛选字段 key，内置字段使用 `base:*`，模板字段使用 `template:*` |
| `label` | string | 字段展示名称 |
| `source` | string enum | `base` 或 `template` |
| `value_type` | string enum | `text`、`number`、`select`、`date`、`file`、`url`、`boolean` 或 `mixed` |
| `values` | array[`FilterValueResponse`] | 当前视角下出现过的值和计数，按 `count DESC, value ASC` 排序 |

`FilterValueResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `value` | string | 后端统一转成字符串的筛选值 |
| `count` | integer | 命中数量；物品筛选值按去重物品计数，入库筛选值按去重入库单计数 |
 
 ---
 
 ## RBAC 权限代码汇总
 
 本业务 API 引入的权限代码：
 
 | 权限代码 | 所属模块 | 说明 |
 |----------|----------|------|
 | `stock.read` | （已有） | 查看库存数据和列表 |
 | `stock.write` | （已有） | 创建或修改库存数据 |
 | `stock.item.manage` | 物品管理 | 创建、修改、删除物品 |
| `stock.inbound.create` | 入库 | 创建入库单 |
| `stock.inbound.approve` | 入库 | 审批或拒绝入库单 |
| `stock.outbound.create` | 出库 | 创建出库单 |
| `stock.outbound.approve` | 出库 | 审批或拒绝出库单 |
 | `stock.template.manage` | 模板管理 | 管理分类模板 |
 | `stock.substitute.manage` | 替代料管理 | 绑定/解绑替代关系 |
 | `audit.read` | 事件日志 | 查看审计日志 |
 
权限分配由用户管理接口直接写入用户权限关系。
首个用户获得全部内置权限；后续用户默认无权限，需要由拥有 `user.permissions.update` 的用户显式分配。
 
 ---
 
## 实现顺序建议

 1. **物品 CRUD**（#1）— 无依赖，基础设施类型的实体管理
 2. **模板管理**（#2 配置前置）— 不依赖物品，但入库需要模板校验；可先做模板，再做入库
 3. **入库 + 模板化校验**（#2 入库）— 依赖物品和模板均已存在
 4. **出库**（#2 出库）— 依赖物品和库存量
 5. **看板统计**（#3）— 依赖出入库数据，统计查询仅读接口
 6. **替代料**（#4）— 依赖物品存在，可并行或稍后实现
 7. **事件日志**（#5）— 可作为贯穿各模块的横切关注点，随业务模块逐步补充实现
 
 每完成一个模块时，同步更新 `docs/code-map.md` 和 `docs/business-api.md` 中的接口记录。
