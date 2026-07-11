# `core/src/persistence/entity/stock_template_field.rs`

本文件定义 `stock_template_fields` 的 SeaORM Entity。

该实体不作为 HTTP 请求体直接接收。字段组合规则由 `stock` 服务层校验，数据库负责字段类型、布尔值、排序和同模板字段名唯一约束。

## 字段约束

| 字段 | 限制 |
| --- | --- |
| `id` | SQLite 自增主键 |
| `template_id` | 必填；外键指向 `stock_templates.id`，模板删除时级联删除字段 |
| `field_name` | 非空文本；同一模板内唯一 |
| `field_type` | 非空文本；数据库 `CHECK` 限制为 `text`、`number`、`select`、`date`、`file`、`url` 或 `boolean` |
| `required` | SQLite 0/1 布尔值 |
| `searchable` | SQLite 0/1 布尔值 |
| `options_json` | 可空；select 候选值 JSON |
| `default_value` | 可空 |
| `sort_order` | 非负整数 |
| `created_at` | SQLite UTC 字符串 |
| `updated_at` | SQLite UTC 字符串 |
