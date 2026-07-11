# Stock HTTP DTO 限制

库存 HTTP 请求统一通过 `ValidatedJson`、`ValidatedPath` 或 `ValidatedQuery` 进入服务层，未知 JSON 字段被拒绝。

## 分类与两类模板

- 分类名称、模板名称和字段名称 trim 后不能为空，并受最大长度限制。
- 模板字段数为 1 至 64，同一模板字段名忽略大小写后唯一。
- `select` 必须提供非空且去重的候选项；其它类型不能提供候选项。
- number、boolean、URL 和 select 默认值必须能被对应类型解释。

## 物品

- `category_id` 是物品分类 ID；`attribute_template_id` 是可选属性预设 ID，二者不互换。
- `attributes` 允许模板字段和自定义字段共存，同一物品字段名唯一。
- 属性值按 text、number、select、date、file、url、boolean 执行类型校验。
- file 值必须是 `{ "file_id": positive_integer }`。

## 入库

- 每条明细独立携带 `inbound_template_id` 和 `ext_attributes`。
- 未显式提供模板时，服务层可以使用物品属性模板的 `default_inbound_template_id`。
- 实际属性按入库模板必填、类型和候选项校验；未知字段被拒绝。
- 数量必须大于 0，单价不能为负，物品和库位必须有效。

## 出库、库位和替代料

既有正数、非负数、层级循环、库存占用、替代料自引用/重复/循环规则保持不变，具体数据库与事务约束见对应 service 和 repository。
