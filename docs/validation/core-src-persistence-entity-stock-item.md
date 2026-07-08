# `core/src/persistence/entity/stock_item.rs`

本文件定义 `stock_items` 的 SeaORM Entity。

该实体不作为 HTTP 请求体直接接收。HTTP 物品输入限制见 `core/src/stock/controller.rs`，写库输入限制见 `core-src-persistence-repository-stock-repo.md`。

## 字段约束

| 字段 | 限制 |
| --- | --- |
| `id` | SQLite 自增主键 |
| `name` | 非空文本；写库前由 repository 输入校验 |
| `sku` | 非空文本；未软删除记录由 `idx_stock_items_sku_active` 保证唯一 |
| `category_id` | 可空；外键指向 `stock_templates.id`，模板删除后置空 |
| `unit` | 非空文本；写库前由 repository 输入校验 |
| `description` | 可空 |
| `default_price` | 可空；数据库 `CHECK` 限制非负 |
| `reorder_point` | 可空；数据库 `CHECK` 限制非负 |
| `created_at` | SQLite UTC 字符串 |
| `updated_at` | SQLite UTC 字符串 |
| `deleted_at` | 可空；为空表示当前有效，非空表示软删除 |
