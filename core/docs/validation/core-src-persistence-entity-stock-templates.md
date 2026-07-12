# 分类与两类模板实体限制

- `stock_item_categories` 只保存分类元数据，未软删除名称唯一。
- `stock_item_attribute_templates` 是可选物品属性预设，可关联默认入库模板。
- `stock_inbound_templates` 只定义单次收货属性。
- 两类模板字段分别保存在独立字段表中，同一模板字段名唯一，字段类型受数据库 CHECK 限制。
- 仅物品模板字段保存 `unit_mode`、`fixed_unit` 和 `unit_options_json`；初始 schema 默认使用 `none`，入库模板字段结构保持不变。
- 模板均采用软删除；实际物品属性和实际入库属性不会因模板软删除而丢失。
