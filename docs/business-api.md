# WineStock 业务 API 文档

本文档是业务 API 文档入口。详细接口按业务域拆分到 `docs/business-api/` 目录。

每个接口的设计风格与当前 Core 中 auth/users 模块一致：

- 统一使用 `POST/GET/PUT/DELETE` + JSON body（`ValidatedJson` + `#[serde(deny_unknown_fields)]`）
- 需要鉴权的接口统一使用 bearer token（`CurrentUser` extractor + `AuthorizeRouteExt`）
- 返回错误统一映射到标准 HTTP 状态码
- DTO 使用 `garde::Validate` + `utoipa::ToSchema`，以支持请求校验和 OpenAPI 输出

当前 RBAC 启动会补齐业务文档列出的库存和审计权限代码。业务授权统一通过 route layer 的 `AuthorizeRouteExt` 判断权限代码。

## 业务文档

| 业务域 | 文档 |
| --- | --- |
| 库存物品 | [`business-api/items.md`](business-api/items.md) |
| 库存模板 | [`business-api/templates.md`](business-api/templates.md) |
| 入库 | [`business-api/inbound.md`](business-api/inbound.md) |
| 出库 | [`business-api/outbound.md`](business-api/outbound.md) |
| 库存审批 | [`business-api/stock-approvals.md`](business-api/stock-approvals.md) |
| 总览看板 | [`business-api/dashboard.md`](business-api/dashboard.md) |
| 替代料 | [`business-api/substitutes.md`](business-api/substitutes.md) |
| 事件日志 | [`business-api/events.md`](business-api/events.md) |

## 支撑文档

- [`business-api/common.md`](business-api/common.md)：分页响应和筛选值响应等通用结构。
- [`business-api/permissions.md`](business-api/permissions.md)：业务权限代码汇总。
- [`business-api/implementation-order.md`](business-api/implementation-order.md)：实现顺序建议。

每完成一个模块时，同步更新 `docs/code-map.md` 和对应的 `docs/business-api/*.md` 文档。
