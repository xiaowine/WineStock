# 入库库位分层预填：全局默认库位 + 同编号历史库位 + 批量设置（2026-07-27，方案确认待实施）

> 实施状态：**已实施**（2026-07-27）——core `is_default`（含测试）、契约同步、库位页星标、
> 入库行三层预填（`useInboundDraft.prefillLineLocation`）与 `InboundBatchLocationDialog`
> 批量设置均已完成；存量测试库 ALTER 与实机验证待做。
> 背景痛点：订单导入/批量创建后，入库明细仍需逐行设置库位。

三层机制按优先级组合，预填等同手选、任何一行可改：

1. **同编号历史库位**（仅严格同一物品，不做任何相似性推断）：该物品当前有库存且
   **只分布在一个库位**时预填该库位；多库位或无库存不猜。
   明确否决"类似物品聚集"启发式（同分类/同封装推荐，第三方 App 的库位画像做法）——
   物品量大时相似性判断不可靠，放错比不填更糟（用户确认）。
2. **全局默认库位**：无历史时回落。沿用 2026-07-26 既定的 `is_default` 全局默认模式
   （物品属性模板已实施，见 `item-template-global-default.md`；本次照抄到库位）。
   典型用法：设"收货区/待整理"库位承接新料。
   既有决定不变：出库库位、筛选、移库目标**不**预选；本预填仅作用于入库明细。
3. **批量设置库位**：入库草稿提供"批量设置库位"操作，把仍为"待选择"的明细一次设为
   所选库位；导入大单时的人工兜底。

交付顺序：先第 2、3 层（全局默认为既定模式复刻、批量设置纯前端草稿操作），
第 1 层紧随其后（需要在装配入库行时查询物品库存分布）。

## core 改动（照 item-template-global-default 模式）

| 位置 | 内容 |
| --- | --- |
| `persistence/migration/m20260706_000001_initial_schema.rs` | `stock_locations` 增列 `is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0,1))`（直接改初始迁移，存量库原地 ALTER） |
| 库位实体 + repository | Model 增 `is_default: bool`；更新支持 `is_default`：置 1 前事务内清除其它默认（软删除默认库位随行失效，无需清理） |
| `stock/controller/locations` | `LocationUpdateRequest` 增可选 `is_default`；`LocationResponse` 增 `is_default: bool`；不加独立端点，走既有 PUT，权限沿用现有库位管理权限 |
| `tests/stock_locations.rs` | 设默认/换默认自动清旧/响应携带/删除默认后无默认 |

**存量测试库原地转换**（桌面 `server` 数据目录 + Android 测试机，经 adb 停止应用后执行）：

```sql
ALTER TABLE stock_locations ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;
```

## 契约同步

`cd frontend && pnpm gen:api`。

## frontend 改动

| 位置 | 内容 |
| --- | --- |
| `LocationsPage` / `components/locations/` | 行内"设为默认"星标与"默认"标记（对照模板页交互）；设/取消默认走既有更新接口 `{ is_default }` |
| 入库草稿装配（`pages/inbound-draft/` 行创建处） | 新行库位预填链：同编号唯一库存库位（第 1 层，装配时查 `getItemInventory`）→ 全局默认库位 → 待选择；对导入订单/扫码/手动添加统一生效 |
| 入库草稿工具区 | "批量设置库位"：选择库位后应用到全部"待选择"明细（已设库位的行不覆盖）；Dialog 复用库位选择控件 |
| 文档 | `page-inbound.md`、`page-locations.md`、code-map 相应条目 |

第 1 层查询成本：逐行装配时查询该物品库存（现有 `GET /api/items/{id}/inventory`），
导入批量行串行/小并发查询；查询失败静默回落第 2 层，不阻塞行创建。

## 明确不做

- 相似物品/分类/封装的库位推荐启发式；
- 出库库位、筛选、移库目标预选（沿用既有决定）；
- 分类全局默认（同模式另行实施）；
- 单据级"本单默认库位"字段（三层已覆盖，避免默认值层级过多）。

## 验证计划

1. `cargo test -p winestock-core`（库位默认用例）；
2. `pnpm gen:api` 后 `vue-tsc` 与前端测试；
3. 浏览器：库位页设/换/取默认；导入订单后明细预填链三种落点；批量设置只填"待选择"行；
   出库/移库/筛选不受影响；
4. 存量库 ALTER 后桌面与 Android 实测。
