# `core/src/persistence/repository/stock_repo.rs` 和 `stock_repo/`

库存仓储以 `stock_repo.rs` 作为模块入口，具体输入模型和查询实现拆分在 `stock_repo/` 子模块中。

## `TemplateFieldInput`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。HTTP 模板字段输入限制见 `core-src-stock-controller.md`。

校验入口：模板创建、更新或复制写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `field_name` | `garde length(min = 1, max = 64)`；trim 后非空；数据库保证同一模板内唯一 |
| `field_type` | `garde length(min = 1, max = 32)`；trim 后非空；数据库 `CHECK` 限制稳定代码集合 |
| `required` | 布尔值，写库时转换为 SQLite 0/1 |
| `searchable` | 布尔值，写库时转换为 SQLite 0/1 |
| `options_json` | 可空；存在时 trim 后非空，最大 4096；服务层负责 JSON 结构和字段类型组合校验 |
| `default_value` | 可空；存在时 trim 后非空，最大 256；服务层负责类型组合校验 |
| `sort_order` | 非负整数 |

## `CreateStockTemplate`

校验入口：`StockRepository::create_template()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `name` | `garde length(min = 1, max = 128)`；trim 后非空；未软删除模板唯一 |
| `description` | 可空；存在时 trim 后非空，最大 1024 |
| `fields` | `garde(dive)`；服务层要求 1 到 64 个字段 |

## `UpdateStockTemplate`

校验入口：`StockRepository::update_template()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `name` | 可不修改；存在时 trim 后非空，最大 128；未软删除模板唯一 |
| `description` | 可不修改；可写入或清空说明 |
| `fields` | 可不修改；存在时整体替换旧字段 |

## `CreateStockItem`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。

校验入口：`StockRepository::create_item()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `name` | `garde length(min = 1, max = 128)`；trim 后非空 |
| `sku` | `garde length(min = 1, max = 64)`；trim 后非空；未软删除记录唯一 |
| `category_id` | 可空；数据库外键约束引用模板 |
| `unit` | `garde length(min = 1, max = 32)`；trim 后非空 |
| `description` | 可空；存在时 trim 后非空，最大 1024 |
| `default_price` | 可空；服务层和数据库均要求非负 |
| `reorder_point` | 可空；服务层和数据库均要求非负 |

## `UpdateStockItem`

该实体由 `stock` 服务层构造。外层 `Option` 表示是否修改字段；内层 `Option` 表示可空字段是否清空。

校验入口：`StockRepository::update_item()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `name` | 可不修改；存在时 trim 后非空，最大 128 |
| `sku` | 可不修改；存在时 trim 后非空，最大 64；未软删除记录唯一 |
| `category_id` | 可不修改；可写入或清空模板关联 |
| `unit` | 可不修改；存在时 trim 后非空，最大 32 |
| `description` | 可不修改；可写入或清空；存在时 trim 后非空 |
| `default_price` | 可不修改；可写入或清空；服务层和数据库均要求非负 |
| `reorder_point` | 可不修改；可写入或清空；服务层和数据库均要求非负 |

## `ListStockItems`

服务层负责把缺省分页归一化为 `page = 1`、`page_size = 50`，并把 `page_size` 限制到最大 200。

## `CreateInboundOrderItem`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。HTTP 入库明细输入限制见 `core-src-stock-controller.md`。

校验入口：`StockRepository::create_inbound_order()` 写库明细前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `item_id` | 必须大于 0；服务层先确认未软删除物品存在，数据库外键仍是最终保护 |
| `quantity` | 有限数字且大于 0 |
| `unit_price` | 有限数字且非负 |
| `location` | 可空；存在时 trim 后非空，最大 128 |
| `batch_no` | 可空；存在时 trim 后非空，最大 128；为空时审批阶段生成内部批次号 |
| `expires_at` | 可空；存在时 trim 后非空，最大 64 |
| `ext_attributes_json` | 可空；存在时 trim 后非空，最大 8192；服务层在审批前校验 JSON object、模板字段名、必填项、类型和选项 |

## `CreateInboundOrder`

校验入口：`StockRepository::create_inbound_order()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `source` | `garde length(min = 1, max = 128)`；trim 后非空 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |
| `created_by_user_id` | 可空；由当前 bearer 用户写入，数据库外键约束用户存在性 |
| `items` | 服务层要求 1 到 256 条；repository 也拒绝空明细 |

## `ListInboundOrders`

服务层负责把缺省分页归一化为 `page = 1`、`page_size = 50`，并把 `page_size` 限制到最大 200。`item_id` 存在时必须大于 0；`date_from`、`date_to` 和 `search` 存在时 trim 后不能为空。

## `CreateOutboundOrderItem`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。HTTP 出库明细输入限制见 `core-src-stock-controller.md`。

校验入口：`StockRepository::create_outbound_order()` 写库明细前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `item_id` | 必须大于 0；服务层先确认未软删除物品存在，数据库外键仍是最终保护 |
| `quantity` | 有限数字且大于 0 |
| `batch_id` | 可空；存在时审批阶段只扣指定批次，批次必须属于该物品且剩余库存足够 |
| `location` | 可空；存在时 trim 后非空，最大 128 |

## `CreateOutboundOrder`

校验入口：`StockRepository::create_outbound_order()` 写库前调用 `validate_repository_input()`。

| 字段 | 限制 |
| --- | --- |
| `destination` | `garde length(min = 1, max = 128)`；trim 后非空 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |
| `created_by_user_id` | 可空；由当前 bearer 用户写入，数据库外键约束用户存在性 |
| `items` | 服务层要求 1 到 256 条；repository 也拒绝空明细 |

## `ListOutboundOrders`

服务层负责把缺省分页归一化为 `page = 1`、`page_size = 50`，并把 `page_size` 限制到最大 200。`item_id` 存在时必须大于 0；`date_from` 和 `date_to` 存在时 trim 后不能为空。

## 看板读取模型

`DashboardOverviewRecord`、`SlowMovingStockItemRecord` 和 `DailyMovementTrendRecord` 都是只读聚合模型，不作为写库输入，不走 `validate_repository_input()`。

- 总览统计从未软删除物品、当前批次剩余数量和 `stock_movements` 聚合读取。
- 近三天出入库和每日趋势只依赖审批流程写入的库存流水，因此不会统计 `pending` 或 `rejected` 单据。
- 呆滞料查询由服务层传入固定阈值 30 天，repository 只按阈值筛选当前有库存的未软删除物品。
- 趋势查询由服务层把 `days` 限制为 1 到 365，repository 负责补齐无流水日期的 0 值。

## `BindStockSubstitute`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。HTTP 替代料输入限制见 `core-src-stock-controller.md`。

校验入口：`StockRepository::replace_substitutes()` 写库前调用 `validate_repository_input()`，并额外检测自引用、重复替代物品、重复优先级和循环绑定。

| 字段 | 限制 |
| --- | --- |
| `substitute_item_id` | 必须大于 0；服务层和 repository 要求指向未软删除物品 |
| `priority` | 必须大于 0；同一主物品提交内不能重复 |
| `notes` | 可空；存在时 trim 后非空，最大 1024 |

## `StockSubstituteRecord`

只读模型，不作为写库输入。读取时只返回未软删除主物品和未软删除替代物品，并聚合替代物品当前批次剩余库存。

## `ListAuditEvents`

该实体由 `stock` 服务层构造，不作为 HTTP 请求体直接接收。HTTP 查询参数限制见 `core-src-stock-controller.md`。

服务层负责把缺省分页归一化为 `page = 1`、`page_size = 50`，并把 `page_size` 限制到最大 200。`entity_id` 和 `user_id` 存在时必须大于 0；文本筛选参数存在时 trim 后不能为空。

## `AuditEventRecord`

只读模型，不作为写库输入。读取时从 `audit_events` 左连接 `auth_users` 获取用户名；`user_id` 为空或用户不存在时 `username` 为空。

## `StockRepository`

写库和查询约束：

- `create_item()` 只创建 `deleted_at IS NULL` 的有效物品。
- `create_template()`、`update_template()` 和 `copy_template()` 在单个事务内写入模板和字段定义。
- `list_active_templates()` 和 `find_active_template_by_id()` 只返回未软删除模板，并按 `sort_order, id` 返回字段。
- `list_active_items()` 只返回未软删除物品，支持名称/SKU 小写模糊搜索和模板筛选。
- `active_sku_exists_except()` 用于服务层在写入前返回稳定的 `sku_taken` 错误；数据库局部唯一索引仍是最终保护。
- `active_template_name_exists_except()` 用于服务层在写入前返回稳定的 `template_name_taken` 错误；数据库局部唯一索引仍是最终保护。
- `active_items_reference_template()` 用于模板软删除前拒绝删除仍被有效物品引用的模板。
- `soft_delete_item()` 只写入 `deleted_at` 和 `updated_at`，不物理删除历史物品记录。
- `soft_delete_template()` 只写入 `deleted_at` 和 `updated_at`，不物理删除历史模板记录。
- `create_inbound_order()` 在单个事务内创建 `pending` 入库单、明细和创建审计事件；创建阶段不写库存批次或库存流水。
- `list_inbound_orders()` 和 `find_inbound_order_by_id()` 返回入库单主表和明细，支持按物品 ID 与创建时间字符串筛选。
- `approve_inbound_order()` 要求单据仍为 `pending`，并在单个事务内更新状态、生成批次、写库存流水和审批审计事件；非 `pending` 返回稳定的 repository 自定义错误供服务层映射为 `409 order_not_pending`。
- `reject_inbound_order()` 要求单据仍为 `pending`，并在单个事务内更新状态和写拒绝审计事件；拒绝不改变库存。
- `create_outbound_order()` 在单个事务内创建 `pending` 出库单、明细和创建审计事件；创建阶段不扣减库存。
- `list_outbound_orders()` 和 `find_outbound_order_by_id()` 返回出库单主表和明细，支持按物品 ID、创建时间字符串和出库历史搜索筛选；`list_outbound_filter_values()` 返回出库历史筛选值，批次和模板值从指定批次或已审批扣减流水反查。
- `approve_outbound_order()` 要求单据仍为 `pending`，并在单个事务内更新状态、按指定批次或 FIFO 扣减批次库存、写库存流水和审批审计事件；库存不足或指定批次不可扣减返回稳定的 repository 自定义错误供服务层映射为 `409 insufficient_stock`，事务整体回滚。
- `reject_outbound_order()` 要求单据仍为 `pending`，并在单个事务内更新状态和写拒绝审计事件；拒绝不扣减库存。
- `dashboard_overview()` 仅读取未软删除物品、当前批次剩余库存和审批后库存流水，返回总览、近三天流转和呆滞料聚合。
- `dashboard_trends()` 按日期升序返回连续趋势数据，无流水日期补 0。
- `replace_substitutes()` 整体替换指定物品的替代料列表，在同一事务内删除旧关系、插入新关系并写 `linked` 审计事件；自引用、重复替代物品、重复优先级或循环绑定返回稳定的 repository 自定义错误供服务层映射为 `400 invalid_request`。
- `list_substitutes()` 返回替代料列表并聚合替代物品当前库存。
- `delete_substitute()` 删除单条替代料关系并写 `unlinked` 审计事件；关系不存在时返回 false。
- `list_audit_events()` 分页读取审计事件，支持按实体类型、实体 ID、动作、用户 ID 和时间字符串范围筛选。
