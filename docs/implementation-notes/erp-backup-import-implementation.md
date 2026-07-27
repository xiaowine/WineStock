# 阶段2：LCSC Android ERP 备份导入实施（2026-07-27，已实施）

> 实施状态：解析纯逻辑（`erp/backupImport.ts`，7 个 node 测试 + 真实备份文件验证通过）、
> 文件读取入口、`ErpBackupImportDialog.vue`（预览/勾选批量创建/执行编排）、
> `useInboundDraft.importBackup` 装配、入库页入口均已完成，构建与浏览器点击级 E2E 通过。
> Chrome DevTools MCP 已使用真实备份完成预览、建库位和草稿装配验证；真机走查待补。

> 承接 `lcsc-batch-item-creation-and-erp-backup-import.md` 的"特性二"。方案决策（备份格式事实、
> C0 跳过、图片在线补、期初语义、重复导入防护、时间戳边界、明确不做）以该文档为准，
> 本文只写落地到文件/函数/交互粒度的实施细化。特性一（批量创建 composable）已实施，本阶段复用。

## 架构分层（对照订单导入）

订单导入是三层：纯解析（`lcsc/orderExport.ts`）→ 文件读取入口（`lcsc/orderExportFile.ts`，
动态 SheetJS）→ 预览 Dialog（`LcscOrderImportDialog.vue`）。备份导入沿用同构，差异在于备份是
**四张关系表**而非单表行数组，且执行阶段有真实副作用（建库位、建物品）。

| 层          | 订单导入                           | 备份导入（本阶段新增）                                                 |
| ----------- | ---------------------------------- | ---------------------------------------------------------------------- |
| 纯解析      | `lcsc/orderExport.ts`              | `erp/backupImport.ts`（四表 → 结构化 + join，无 SheetJS，node 可测）   |
| 文件读取    | `lcsc/orderExportFile.ts`          | `erp/backupImportFile.ts`（动态 SheetJS，按 sheet 名取行数组交纯解析） |
| 预览 Dialog | `LcscOrderImportDialog.vue`        | `components/stock-draft/ErpBackupImportDialog.vue`                     |
| 批量创建    | `useBatchLcscItemCreation`         | 复用同一 composable                                                    |
| 草稿装配    | `useInboundDraft.importOrderLines` | `useInboundDraft.importBackup`                                         |

## 纯解析契约（`erp/backupImport.ts`）

输入：`{ name: string; rows: (string|number|null)[][] }[]`（各 sheet 的 `sheet_to_json(header:1)` 行数组，
与订单导入同形）。按 **sheet 名 + 表头名**取列，不依赖列顺序。

```ts
export interface ErpBackupLocation { code: string; displayName: string | null }
export interface ErpBackupComponent {
  id: number; partNumber: string;          // 已大写归一
  name: string | null; brand: string | null; packageName: string | null;
  category: string | null; description: string | null;
  isManual: boolean;                        // partNumber 形如 ^C0\d+$
}
export interface ErpBackupInventory { componentId: number; locationId: number; quantity: number }

export interface ErpBackupParseResult =
  | { ok: true;
      appVersion: string | null;            // meta.appVersionName，进单据备注
      locations: ErpBackupLocation[];       // 按 code 去重后的全部有效库位
      /** 组装好的导入项：真 C 码器件 + 数量 + 库位 code；已 join、已剔除数量<=0。 */
      items: Array<{ component: ErpBackupComponent; locationCode: string | null; quantity: number }>;
      /** C0 手工器件及其数量，仅用于跳过提示与计数。 */
      skippedManual: Array<{ partNumber: string; quantity: number }>;
    }
  | { ok: false; error: string };
```

解析规则：

- `meta` 表：读 `schemaVersion`，`!= 1` 直接 `{ ok:false }`（提示不支持的备份版本）；读 `appVersionName` 存备注。
- `storage_locations`：表头 `id/code/displayName`；`id → {code, displayName}`。code 空则跳过该库位行；
  所有有效库位都会进入 `locations`，包括当前没有库存引用的空库位。
- `components`：表头 `id/partNumber/name/brand/packageName/category/description`；partNumber 归一大写；
  `isManual = /^C0\d+$/.test(partNumber)`。
- `inventory_items`：表头 `componentId/locationId/quantity`；数值单元格为 double，按整数解析；
  `quantity <= 0` 跳过。
- **join**：inventory_items → components（缺失或 componentId 无对应则跳过该库存行）→ storage_locations
  （locationId 无对应则 locationCode=null，执行阶段落全局默认库位/待选择）。
- **C0 分流**：component.isManual 的库存行进 `skippedManual`（累计数量），不进 `items`。
- 四表缺失关键表（storage_locations/components/inventory_items 任一无表头）→ `{ ok:false }`。

数值 double→int：`Math.round`；负/NaN 视为无效跳过。

## 执行阶段（Dialog 内，用户确认后）

预览确认后按序执行，任一步失败给 Notice 并中止（已建的库位/物品保留，草稿不写）：

1. **库位落地**：`locations` 逐个按 `name === code` 匹配现有库位（`listLocations`）；
   缺失的用 `createLocation({ group_id: 示例库区/首个根分组, name: code })` 新建，建立
   `code → locationId` 映射。库位新建串行，失败即停并提示。
2. **物品批量创建**：`items` 中未匹配现有 C 码的（`listItemOptions` 精确匹配）走
   `useBatchLcscItemCreation.run(codes, options)`（弹批次选项对话框，同特性一，含勾选可分批）；
   已匹配的直接复用。得到 `partNumber → ItemOptionResponse` 映射。
3. **组装期初草稿**：`items` → 草稿行 `{ item, quantity, unitPrice: 0, locationId }`（locationCode 经步骤1
   映射为 id；null 时留待预填/批量设置）。经 `emit("import", payload)` 交入库装配。

装配侧 `useInboundDraft.importBackup(payload)`：

- 写入草稿行（复用 addItem + 逐行设 quantity/unitPrice/locationId）；
- `source` 预填 `"备份导入 <文件名>"`；`notes` 预填 `导出版本 <appVersion> · 跳过手工器件 N 项`；
- **重复导入防护**：写入前查 `listInbound({ search: "备份导入" })` 或本地已有草稿来源，命中则
  Notice 警告"该备份可能已导入过，再次导入会使数量翻倍"，由用户在 Dialog 内确认继续。

## 预览交互（`ErpBackupImportDialog.vue`）

- 文件选择（`.xlsx`）→ 解析 → 三段预览：
  - **库位**：N 个库位，标注"已存在 / 待新建"；
  - **物品**：真 C 码行，复用订单导入的匹配/未匹配/勾选批量创建 UI（可直接复用 Dialog 的行渲染结构）；
  - **跳过**：C0 手工器件（灰显不可选，徽标"手工器件"），顶部提示文案（方案文档已定）。
- 主操作："导入 N 项到入库草稿"（执行三阶段）；批量创建入口沿用特性一（勾选 + 创建选中）。
- 关闭中止安全；解析失败在 `backupImportFile.ts` 唯一入口记 `trackTelemetryIssue`。

## 入口

入库页操作区（仅入库域）在"导入订单"旁增"导入备份"按钮 → 打开 `ErpBackupImportDialog`。
与订单导入并列，二者独立。

## 复用点（不重复造）

- SheetJS 动态加载：照抄 `orderExportFile.ts` 模式（不进主包）。
- 批量创建：`useBatchLcscItemCreation`（含勾选分批、串行限速、失败重试）。
- 草稿写入：`useInboundDraft` 的 addItem + 库位三层预填（备份行有 locationId 时直接用，
  无时回落预填）。
- 库位新建：现有 `createLocation`。

## 明确不做（承接方案）

- 解析备份内嵌浮动图片（SheetJS CE 读不到）——真 C 码在线补图；
- C0 手工器件导入；伪造历史流水时间；core 侧新接口。

## 测试

- `tests/erpBackupImport.test.mjs`（node）：四表解析、表头匹配、double→int、C0 分流、
  join 剔除孤儿行、空库位保留、schema 版本门、缺表报错；用最小构造行数组，不依赖真实文件。
- 构建与 Chrome DevTools MCP 浏览器 E2E 使用真实备份
  `C:\Users\xiaow\Desktop\lcsc_inventory_backup_0727.xlsx`：预览识别 5 个库位、3 个器件和 3 条库存；
  5 个库位创建均返回 201，随后重新加载库位选项，草稿立即正确显示 J1/D1/R1，未出现“库位已失效”。
- 移动端在 360、390、412 px 宽度验证入库和出库操作区：按钮保持两列，标题、操作区和页面均无
  横向溢出；共享入口显示“选择物品”，浏览器控制台无 warning/error/issue。

## 交付顺序

解析纯逻辑 + node 测试 → 文件读取入口 → Dialog + 装配接入 → 文档同步 + E2E。
