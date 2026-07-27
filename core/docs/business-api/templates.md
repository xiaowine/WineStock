# 分类与属性模板 API

分类与物品属性模板是两个独立概念：分类只负责归类；物品属性模板是可选录入预设。系统不提供旧的统一模板接口。

共同权限：读取使用 `stock.template.read`，创建、修改、复制和删除使用 `stock.template.manage`。

## 物品分类

- `POST /api/item-categories`
- `GET /api/item-categories`
- `GET /api/item-categories/{id}`
- `PUT /api/item-categories/{id}`
- `DELETE /api/item-categories/{id}`

分类响应还包含 `item_usage_count`：当前未软删除且直接关联该分类的物品数量。分类不包含字段定义，也不决定入库表单。删除成功返回 `200` 与 `{ "affected_active_item_count": n }`；该数量在删除事务内计算，表示仍保留失效分类引用的当前有效物品数。

## 物品属性模板

- `POST /api/item-attribute-templates`
- `GET /api/item-attribute-templates`
- `GET /api/item-attribute-templates/{id}`
- `PUT /api/item-attribute-templates/{id}`
- `DELETE /api/item-attribute-templates/{id}`
- `POST /api/item-attribute-templates/{id}/copy`

请求包含名称、说明和字段数组。模板字段只用于快速生成物品属性；物品可以不选择模板，也可以在模板字段之外增加任意自定义属性。

物品模板字段额外包含 `catalog_visible`。创建或更新时未显式提供该值的字段按数组顺序默认将前三个设为 `true`；显式布尔值保持调用方选择。每个模板最多三个字段可以设为 `true`，目录按字段现有 `sort_order` 返回对应物品值；物品私有自定义属性固定不可作为目录字段。

物品属性模板响应还包含 `item_usage_count`：当前未软删除且直接关联该模板的物品数量。删除物品属性模板会删除模板字段定义及其对应的物品属性值，并把现有物品的 `attribute_template_id` 置空；不会因仍有物品引用而返回 `409 template_in_use`。删除成功返回 `200` 与 `{ "affected_active_item_count": n }`，该数量在删除事务内计算。调用方必须把该操作作为会造成业务数据丢失的高风险删除明确提示。

## 模板字段

| 字段 | 说明 |
|---|---|
| `field_name` | 模板内唯一字段名称 |
| `field_type` | `text`、`number`、`select`、`date`、`file`、`url` 或 `boolean` |
| `required` | 是否必填 |
| `searchable` | 是否进入筛选值聚合 |
| `options` | `select` 的候选字符串数组 |
| `default_value` | 可选默认值 |

`date` 默认值必须是实际存在的公历 `YYYY-MM-DD` 日期。`file` 固定为单张 PNG、JPEG 或 WebP 图片引用，实际属性值必须是 `{ "file_id": integer }`，不能使用客户端路径或普通字符串；模板本身不能预设某个文件默认值。

## 物品模板单位规则

物品属性模板字段额外且必须返回 `unit` 对象；创建和更新请求省略 `unit` 时按 `none` 处理。

| `unit.mode` | 模板定义 | 物品录入规则 |
|---|---|---|
| `none` | `value`、`options` 均为空 | 不显示单位控件，服务端不保存单位 |
| `fixed` | `value` 是指定单位，`options` 为空 | 配置端与规则选择器同行输入，录入端只读展示，服务端写入指定值 |
| `select` | `value` 为空，`options` 是不区分大小写去重的单位数组 | 必须选择候选项之一 |

固定单位和单位候选项裁剪首尾空白后长度为 1 至 32；候选项数量为 1 至 32。规则组合不合法时返回 `400 invalid_request`。复制物品属性模板时必须完整复制单位规则。

本地服务首次启动会分别补齐三个分类和三套物品属性预设，内置物品模板默认把前三个字段设为目录展示字段。同名记录已经存在时跳过，不覆盖用户修改，也不恢复软删除记录。分类列表按 `sort_order` 返回，物品属性模板列表按创建顺序返回；两组内置数据都以元器件相关项排在第一位。
