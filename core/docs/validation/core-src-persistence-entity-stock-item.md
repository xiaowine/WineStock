# Stock Item 与属性实体限制

- `stock_items.category_id` 外键指向物品分类。
- `stock_items.attribute_template_id` 外键指向可选物品属性模板。
- `stock_items.image_file_id` 必填且唯一，外键指向受控图片文件对象；数据库不保存客户端或绝对路径。
- SKU 在未软删除物品内唯一；参考单价和再订货点不能为负。
- `stock_item_attribute_definitions` 负责字段名、类型、候选项、单位规则和模板目录展示标记；同一模板或私有物品内字段名忽略大小写唯一，每个模板最多三项 `catalog_visible`，私有定义固定关闭。`stock_item_attributes` 只保存定义引用和值，`value_json` 由 garde 和服务层保证为合法类型化 JSON。
- 属性值必须引用 `definition_id`；自定义定义的 `owner_item_id` 指向所属物品，模板定义的 `template_id` 指向所属模板。
- `storage_item_file_bindings` 对文件 ID 和物品属性 ID 都执行唯一约束。
