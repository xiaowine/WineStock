# `core/src/persistence/entity/stock_template.rs`

本文件定义 `stock_templates` 的 SeaORM Entity。

该实体不作为 HTTP 请求体直接接收。HTTP 模板输入限制见 `core-src-stock-controller.md`，写库输入限制见 `core-src-persistence-repository-stock-repo.md`。

## 字段约束

| 字段 | 限制 |
| --- | --- |
| `id` | SQLite 自增主键 |
| `name` | 非空文本；未软删除记录由 `idx_stock_templates_name_active` 保证唯一 |
| `description` | 可空 |
| `created_at` | SQLite UTC 字符串 |
| `updated_at` | SQLite UTC 字符串 |
| `deleted_at` | 可空；为空表示当前有效，非空表示软删除 |
