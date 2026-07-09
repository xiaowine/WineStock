# 库存模板 API

入库天然是模板化的：物品分类决定了入库时需填写的扩展字段。
模板管理是入库的配置前置，不属于独立业务领域。

当前实现状态：已实现 `POST /api/templates`、`GET /api/templates`、`GET /api/templates/{id}`、`PUT /api/templates/{id}`、`DELETE /api/templates/{id}` 和 `POST /api/templates/{id}/copy`，并纳入 OpenAPI。

本地服务启动后会补齐内置模板：`元器件`、`3D打印耗材` 和 `通用`。补齐只在不存在同名模板记录时创建，不覆盖用户修改，也不恢复用户已经软删除的同名模板。

### `POST /api/templates`


创建新的分类模板。

- 权限：`stock.template.manage`

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 模板名称 |
| `description` | string | 否 | 说明 |
| `fields` | array | 是 | 模板字段定义列表 |

**字段定义：`TemplateFieldDef`**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `field_name` | string | 是 | 字段名称 |
| `field_type` | string (enum) | 是 | 字段类型：`text` / `number` / `select` / `date` / `file` / `url` / `boolean` |
| `required` | boolean | 否 | 是否必填，默认 false |
| `searchable` | boolean | 否 | 是否可用于筛选，默认 false |
| `options` | array[string] | 否 | 当 `field_type` 为 `select` 时，预置可选值 |
| `default_value` | string | 否 | 默认值 |

- 响应：`201` + `TemplateResponse`
- 错误：`400` 名称重复或字段定义不合法

### `GET /api/templates`


模板列表。

- 权限：`stock.template.read`
- 响应：`200` + `Vec<TemplateResponse>`

### `GET /api/templates/{id}`


模板详情，含字段定义。

- 权限：`stock.template.read`

### `PUT /api/templates/{id}`


更新模板定义。更新后只会影响新入库单，不会回填已有物品扩展属性。

- 权限：`stock.template.manage`

### `DELETE /api/templates/{id}`


删除模板（软删除）。

- 权限：`stock.template.manage`
- 错误：`404` 模板不存在 / `409` 仍有未删除物品关联此模板时拒绝

### `POST /api/templates/{id}/copy`


复制模板。

- 权限：`stock.template.manage`
- 请求：`{ "name": string }`（必填；trim 后非空，未软删除模板内唯一）
- 响应：`201` + `TemplateResponse`
- 错误：`404` 源模板不存在 / `409` 新模板名称已被未删除模板占用
