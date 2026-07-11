# Stock Item 与属性实体限制

- `stock_items.category_id` 外键指向物品分类。
- `stock_items.attribute_template_id` 外键指向可选物品属性模板。
- `stock_items.image_file_id` 必填且唯一，外键指向受控图片文件对象；数据库不保存客户端或绝对路径。
- SKU 在未软删除物品内唯一；参考单价和再订货点不能为负。
- `stock_item_attributes` 同一物品字段名唯一，类型受数据库 CHECK 限制，`value_json` 由 garde 和服务层保证为合法类型化 JSON。
- 自定义属性的 `template_field_id` 为空；模板字段来源删除后允许置空。
- `storage_item_file_bindings` 对文件 ID 和物品属性 ID 都执行唯一约束。
