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

## 模板与物品

- `POST /api/templates`
- `GET /api/templates`
- `GET /api/templates/{id}`
- `PUT /api/templates/{id}`
- `DELETE /api/templates/{id}`
- `POST /api/templates/{id}/copy`
- `POST /api/items`
- `GET /api/items`
- `GET /api/items/filter-values`
- `GET /api/items/{id}`
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
