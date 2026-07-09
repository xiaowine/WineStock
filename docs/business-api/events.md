# 事件日志 API

审计和操作追溯记录。

当前实现状态：已实现 `GET /api/events`，并纳入 OpenAPI。事件日志读取 `audit_events`，支持按实体、动作、用户和时间范围筛选，按 `timestamp DESC, id DESC` 返回。

## 所需权限


- `audit.read` — 查看事件日志

### `GET /api/events`


分页查询事件日志。

- 权限：`audit.read`

**查询参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `page` | integer | 页码，默认 1 |
| `page_size` | integer | 每页条数，默认 50 |
| `entity_type` | string | 筛选实体类型（item / inbound / outbound / template / user / substitute） |
| `entity_id` | integer | 筛选实体 ID |
| `action` | string | 操作类型（created / updated / deleted / approved / rejected / linked / unlinked） |
| `user_id` | integer | 操作人 |
| `date_from` | string (datetime) | 起始时间 |
| `date_to` | string (datetime) | 结束时间 |

**响应：`PaginatedResponse<EventLogResponse>`**

`EventLogResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | integer | 日志 ID |
| `timestamp` | string (datetime) | 操作时间 |
| `user_id` | integer | 操作人 ID |
| `username` | string/null | 操作人用户名；用户外键为空时返回 null |
| `entity_type` | string | 实体类型 |
| `entity_id` | integer | 实体 ID |
| `action` | string | 操作类型 |
| `details` | object/null (json) | 变更详情（JSON 格式，记录前后差异或关键摘要） |
