# Stock Repository 输入限制

所有写库输入在事务开始前或逐行写入前调用 `validate_repository_input()`。

## JSON 校验

`core/src/validation.rs` 提供 `validate_json_text` 和 `validate_optional_json_text`，专门供 garde 校验 JSON 字符串语法。`value_json` 和模板候选值 JSON 在进入数据库前必须是合法 JSON。

## 物品与属性

- `CreateStockItem` 同时包含基础资料和 `ItemAttributeInput` 数组。
- `UpdateStockItem.attributes` 存在时整体替换属性。
- 属性名、类型、JSON 值、单位和排序均有静态约束；file 属性同时携带文件 ID 和所有者 ID。
- 物品、属性、审计和文件绑定共享同一事务。

## 入库与属性

- `CreateInboundOrderItem` 保存固定字段、可选 `inbound_template_id` 和 `InboundAttributeInput` 数组。
- 实际属性逐行保存，不在入库明细表保留 JSON 数据列。
- 入库单、明细、属性、文件绑定和审计共享同一事务。

## 模板与分类

分类、物品属性模板和入库模板使用独立输入类型及仓储方法。物品属性模板可保存默认入库模板 ID，并在字段行保存显式单位模式、固定单位或单位候选项；入库模板字段写入独立表，不持久化这些物品专属单位列。
