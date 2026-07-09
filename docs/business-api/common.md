# 通用响应结构

## 分页响应通用结构


`PaginatedResponse<T>`

| 字段 | 类型 | 说明 |
|------|------|------|
| `items` | array[T] | 当前页数据 |
| `total` | integer | 总记录数 |
| `page` | integer | 当前页码 |
| `page_size` | integer | 每页条数 |
| `total_pages` | integer | 总页数 |

## 筛选值响应通用结构

`FilterValuesResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `fields` | array[`FilterFieldResponse`] | 可用于当前列表筛选的字段集合 |

`FilterFieldResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `key` | string | 稳定筛选字段 key，内置字段使用 `base:*`，模板字段使用 `template:*` |
| `label` | string | 字段展示名称 |
| `source` | string enum | `base` 或 `template` |
| `value_type` | string enum | `text`、`number`、`select`、`date`、`file`、`url`、`boolean` 或 `mixed` |
| `values` | array[`FilterValueResponse`] | 当前视角下出现过的值和计数，按 `count DESC, value ASC` 排序 |

`FilterValueResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `value` | string | 后端统一转成字符串的筛选值 |
| `count` | integer | 命中数量；物品筛选值按去重物品计数，入库/出库筛选值按去重单据计数 |
