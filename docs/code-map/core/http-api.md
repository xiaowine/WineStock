# Core 公共 HTTP 接口

本文只列出当前公共路径。请求/响应字段、权限和错误见对应业务 API 文档与 OpenAPI。

## 鉴权与用户

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `POST /api/auth/me/password`
- `GET /api/users`
- `GET /api/users/{id}`
- `PATCH /api/users/{id}/status`
- `PUT /api/users/{id}/permissions`
- `POST /api/users/{id}/password`
- `GET /api/permissions`

## 全局 HTTP

- `GET /api/health`
- `GET /api-docs/openapi.json`
- `/swagger-ui`

## 受控图片文件

- `POST /api/files/images`
  - 物品管理或入库创建权限任一满足即可上传，后续绑定决定读取权限。
- `GET /api/files/{id}`
- `DELETE /api/files/{id}`

## 分类、模板与物品

- `/api/item-categories` 与 `/api/item-categories/{id}`：分类 CRUD。
- `/api/item-attribute-templates`、`/{id}`、`/{id}/copy`：物品属性预设 CRUD/copy。
- `/api/inbound-templates`、`/{id}`、`/{id}/copy`：入库模板 CRUD/copy；两个读取接口按 stock.inbound.create 或 stock.template.read 任一权限放行。
- `POST /api/items`
- `GET /api/items`：物品目录分页、搜索、库存状态、分类、模板与结构化字段筛选；动态 `filters` 使用稳定字段 key 和参数化精确匹配。
- `GET /api/items/options`：轻量选择额外返回推荐入库模板 ID 与可用状态。
- `GET /api/items/filter-values`：按当前目录上下文返回单位、有效库位和可搜索模板属性的分面候选值与计数。
- `GET /api/items/{id}`
- `GET /api/items/{id}/inventory`
- `GET /api/items/{id}/batches`
- `PUT /api/items/{id}`
- `DELETE /api/items/{id}`

## 库位与移库

- `GET /api/location-groups/tree`
- `POST /api/location-groups`
- `PUT /api/location-groups/{id}`
- `DELETE /api/location-groups/{id}`
- `GET /api/locations`
- `POST /api/locations`
- `PUT /api/locations/{id}`
- `DELETE /api/locations/{id}`
- `POST /api/location-transfers`

## 入库、出库与审批

- `POST /api/inbound`
- `GET /api/inbound`
- `GET /api/inbound/filter-values`
- `GET /api/inbound/{id}`
- `POST /api/outbound`
- `GET /api/outbound`
- `GET /api/outbound/filter-values`
- `GET /api/outbound/{id}`
- `POST /api/stock-approvals/inbound/{id}/approve`
- `POST /api/stock-approvals/inbound/{id}/reject`
- `POST /api/stock-approvals/outbound/{id}/approve`
- `POST /api/stock-approvals/outbound/{id}/reject`

## 替代料、看板与事件

- `GET /api/substitutes`
- `GET /api/substitutes/{item_id}`
- `PUT /api/substitutes/{item_id}`
- `DELETE /api/substitutes/{item_id}/{substitute_item_id}`
- `GET /api/dashboard/overview`
- `GET /api/dashboard/trends`
- `GET /api/events`
