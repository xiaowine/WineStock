# 分类与属性模板 API

分类、物品属性模板和入库模板是三个独立概念：分类只负责归类；物品属性模板是可选录入预设；入库模板只描述本次收货状态。系统不提供旧的统一模板接口。

共同权限：读取使用 `stock.template.read`，创建、修改、复制和删除使用 `stock.template.manage`。

## 物品分类

- `POST /api/item-categories`
- `GET /api/item-categories`
- `GET /api/item-categories/{id}`
- `PUT /api/item-categories/{id}`
- `DELETE /api/item-categories/{id}`

分类字段包括 `name`、可选 `description` 和 `sort_order`。分类不包含字段定义，也不决定入库表单。

## 物品属性模板

- `POST /api/item-attribute-templates`
- `GET /api/item-attribute-templates`
- `GET /api/item-attribute-templates/{id}`
- `PUT /api/item-attribute-templates/{id}`
- `DELETE /api/item-attribute-templates/{id}`
- `POST /api/item-attribute-templates/{id}/copy`

请求包含名称、说明、可选 `default_inbound_template_id` 和字段数组。模板字段只用于快速生成物品属性；物品可以不选择模板，也可以在模板字段之外增加任意自定义属性。

仍有有效物品引用物品属性模板时，删除返回 `409 template_in_use`。

## 入库模板

- `POST /api/inbound-templates`
- `GET /api/inbound-templates`
- `GET /api/inbound-templates/{id}`
- `PUT /api/inbound-templates/{id}`
- `DELETE /api/inbound-templates/{id}`
- `POST /api/inbound-templates/{id}/copy`

入库模板只定义包装状态、实收重量、质检结果、收货照片、合格证和批次备注等本次收货字段。历史入库属性按实际属性行保留，不依赖模板继续有效。

## 模板字段

两类属性模板复用相同字段格式：

| 字段 | 说明 |
|---|---|
| `field_name` | 模板内唯一字段名称 |
| `field_type` | `text`、`number`、`select`、`date`、`file`、`url` 或 `boolean` |
| `required` | 是否必填 |
| `searchable` | 是否进入筛选值聚合 |
| `options` | `select` 的候选字符串数组 |
| `default_value` | 可选默认值 |

`date` 默认值必须是实际存在的公历 `YYYY-MM-DD` 日期。`file` 固定为单张 PNG、JPEG 或 WebP 图片引用，实际属性值必须是 `{ "file_id": integer }`，不能使用客户端路径或普通字符串；模板本身不能预设某个文件默认值。

本地服务首次启动会分别补齐三个分类、三套物品属性预设和三套入库模板。同名记录已经存在时跳过，不覆盖用户修改，也不恢复软删除记录。
