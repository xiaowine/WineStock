# 总览看板 API

库存全局统计摘要数据接口，供前端仪表盘消费。

当前实现状态：已实现 `GET /api/dashboard/overview` 和 `GET /api/dashboard/trends`，并纳入 OpenAPI。看板统计只读取当前批次剩余库存和审批后生成的 `stock_movements`，不会把 `pending` 或 `rejected` 单据计入出入库数量。

## 所需权限


- `stock.dashboard.read` — 查看看板总览和趋势

### `GET /api/dashboard/overview`


库存总览卡片数据。

- 权限：`stock.dashboard.read`

**响应：`DashboardOverviewResponse`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_items` | integer | 库存物品种类数 |
| `total_quantity` | number | 总件数 |
| `total_value` | number | 当前批次剩余数量乘以批次成本后的库存总价值 |
| `inbound_3d` | number | 近 3 天入库总数 |
| `outbound_3d` | number | 近 3 天出库总数 |
| `slow_moving_items` | array | 呆滞料列表（当前有库存且 30 天内无出入库流水的物品） |

**呆滞料条目：`SlowMovingItem`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `item_id` | integer | 物品 ID |
| `item_name` | string | 物品名称 |
| `quantity` | number | 当前库存量 |
| `value` | number | 库存价值 |
| `days_since_last_movement` | integer | 最近一次出入库距今的天数 |

### `GET /api/dashboard/trends`


出入库趋势数据，用于近 30 天或自定义范围的可视化。

- 权限：`stock.dashboard.read`
- 查询参数：`days`（默认 30，最大 365；小于 1 时按 1 处理）

**响应：`TrendsResponse`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `daily` | array[DailyTrend] | 每日入库/出库统计数据 |

**`DailyTrend`**

| 字段 | 类型 | 说明 |
|------|------|------|
| `date` | string (date) | 日期 |
| `inbound_quantity` | number | 入库数量 |
| `outbound_quantity` | number | 出库数量 |
