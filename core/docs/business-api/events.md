# 事件日志 API

审计和操作追溯记录。

当前实现状态：已实现 `GET /api/events`，并纳入 OpenAPI。事件日志读取 `audit_events`，支持按实体、动作、用户和时间范围筛选，按 `timestamp DESC, id DESC` 返回。

## 当前写入范围

当前服务会写入以下审计事件：

- `item`：创建、更新和软删除物品，动作分别为 `created`、`updated`、`deleted`；更新详情包含关键字段前后快照和变更字段列表。
- `item_category`：创建、更新和软删除物品分类，动作分别为 `created`、`updated`、`deleted`。
- `item_attribute_template`：创建、更新、复制和软删除模板；复制模板复用创建流程并记录为新模板的 `created`。
- `user`：注册新用户、更新状态、替换权限、管理员设置临时密码、自助改密和软删除用户；注册使用 `created`，软删除使用 `deleted`，其它用户变更使用 `updated`。
- `inbound`：创建、审批通过和驳回入库单，动作分别为 `created`、`approved`、`rejected`。
- `outbound`：创建、审批通过和驳回出库单，动作分别为 `created`、`approved`、`rejected`。
- `location_group`：创建、更新、移动和软删除库位分组，动作分别为 `created`、`updated`、`moved`、`deleted`。
- `location`：创建、更新和软删除库位，动作分别为 `created`、`updated`、`deleted`。
- `location_transfer`：整批次移库，动作为 `created`，详情包含批次、物品、原库位、目标库位和数量。
- `substitute`：整体替换或新增替代料使用 `linked`，清空替代料或删除单条关系使用 `unlinked`；整体替换详情包含旧列表、新列表、实际新增和实际移除的物品 ID。

登录、刷新 token、登出、启动补齐默认模板、RBAC 补齐和鉴权启动设置不写入业务审计事件。

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
| `entity_type` | string | 筛选实体类型（item / item_category / item_attribute_template / user / inbound / outbound / location_group / location / location_transfer / substitute） |
| `entity_id` | integer | 筛选实体 ID |
| `action` | string | 操作类型（created / updated / deleted / approved / rejected / linked / unlinked / moved） |
| `user_id` | integer | 操作人 |
| `date_from` | string (datetime) | 起始时间 |
| `date_to` | string (datetime) | 结束时间 |

**响应：`PaginatedResponse<EventLogResponse>`**

`EventLogResponse`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | integer | 日志 ID |
| `timestamp` | string (datetime) | 操作时间 |
| `user_id` | integer/null | 操作人 ID；用户外键为空时返回 null |
| `username` | string/null | 操作人用户名；用户外键为空时返回 null |
| `entity_type` | string | 实体类型 |
| `entity_id` | integer/null | 实体 ID；不关联具体记录时返回 null |
| `action` | string | 操作类型 |
| `details` | JSON value | 变更详情；历史记录可能是对象、数组、标量或 null |
