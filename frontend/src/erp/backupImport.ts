// 本文件拥有第三方「LCSC Android ERP」xlsx 备份的解析纯逻辑（四表 → 结构化 + join）；
// 它不读取文件、不依赖 SheetJS，也不执行库位/物品创建或草稿写入。
// 备份格式事实见 docs/implementation-notes/lcsc-batch-item-creation-and-erp-backup-import.md。

/** 单个工作表的原始行；SheetJS `sheet_to_json(header:1)` 的输出形状。 */
export type ErpBackupSheetRows = (string | number | null)[][];

/** 一张命名工作表。 */
export interface ErpBackupSheet {
  name: string;
  rows: ErpBackupSheetRows;
}

/** 备份中的库位；WineStock 无 code 字段，导入时以 code 作库位名匹配/新建。 */
export interface ErpBackupLocation {
  code: string;
  displayName: string | null;
}

/** 备份中的器件资料快照。 */
export interface ErpBackupComponent {
  id: number;
  /** 立创 C 码，已大写归一。 */
  partNumber: string;
  name: string | null;
  brand: string | null;
  packageName: string | null;
  category: string | null;
  description: string | null;
  /** partNumber 形如 C0<自增序号>，是原软件手工录入的本地伪码，不对应立创商品。 */
  isManual: boolean;
}

/** 已 join 的一条待导入库存：真 C 码器件 + 库位 code + 数量。 */
export interface ErpBackupImportItem {
  component: ErpBackupComponent;
  /** 对应库位 code；备份 locationId 无对应库位时为 null（执行阶段回落预填）。 */
  locationCode: string | null;
  quantity: number;
}

/** 被跳过的 C0 手工器件（仅用于提示与计数）。 */
export interface ErpBackupSkippedManual {
  partNumber: string;
  quantity: number;
}

export type ErpBackupParseResult =
  | {
      ok: true;
      /** meta.appVersionName，进单据备注。 */
      appVersion: string | null;
      /** 备份中的全部有效库位；按 code 去重。 */
      locations: ErpBackupLocation[];
      /** 组装好的导入项（已 join、已剔除数量<=0 与孤儿行、已分流 C0）。 */
      items: ErpBackupImportItem[];
      /** C0 手工器件及数量。 */
      skippedManual: ErpBackupSkippedManual[];
    }
  | { ok: false; error: string };

const MANUAL_PART_PATTERN = /^C0\d+$/;

/** 解析备份工作簿。四张表按 sheet 名 + 表头名取列，不依赖列顺序。 */
export function parseErpBackupSheets(sheets: ErpBackupSheet[]): ErpBackupParseResult {
  const sheetByName = new Map(sheets.map((sheet) => [sheet.name, sheet.rows]));

  const meta = readMeta(sheetByName.get("meta"));
  if (meta.schemaVersion !== 1) {
    return { ok: false, error: "不支持的备份版本，请使用 schemaVersion 为 1 的备份文件。" };
  }

  const locationRows = sheetByName.get("storage_locations");
  const componentRows = sheetByName.get("components");
  const inventoryRows = sheetByName.get("inventory_items");
  if (!locationRows || !componentRows || !inventoryRows) {
    return { ok: false, error: "备份缺少库位、器件或库存表，无法导入。" };
  }

  const locationsById = readLocations(locationRows);
  const componentsById = readComponents(componentRows);
  if (componentsById.size === 0) {
    return { ok: false, error: "备份中没有器件数据。" };
  }

  const items: ErpBackupImportItem[] = [];
  const skippedManualByPart = new Map<string, number>();

  for (const row of readInventoryRows(inventoryRows)) {
    const component = componentsById.get(row.componentId);
    if (!component) continue; // 孤儿库存行：器件已不在备份内，跳过。
    if (row.quantity <= 0) continue;

    if (component.isManual) {
      skippedManualByPart.set(
        component.partNumber,
        (skippedManualByPart.get(component.partNumber) ?? 0) + row.quantity,
      );
      continue;
    }

    const location = locationsById.get(row.locationId) ?? null;
    items.push({
      component,
      locationCode: location?.code ?? null,
      quantity: row.quantity,
    });
  }

  if (items.length === 0 && skippedManualByPart.size === 0) {
    return { ok: false, error: "备份中没有可导入的库存记录。" };
  }

  const locations = [...locationsById.values()];
  const skippedManual = [...skippedManualByPart.entries()].map(([partNumber, quantity]) => ({
    partNumber,
    quantity,
  }));

  return { ok: true, appVersion: meta.appVersionName, locations, items, skippedManual };
}

interface MetaValues {
  schemaVersion: number | null;
  appVersionName: string | null;
}

/** meta 表是 key/value 两列；按首列键名取值，与列顺序无关。 */
function readMeta(rows: ErpBackupSheetRows | undefined): MetaValues {
  const values = new Map<string, string>();
  for (const row of rows ?? []) {
    const key = cellText(row[0]).trim();
    if (key) values.set(key, cellText(row[1]).trim());
  }
  const rawVersion = values.get("schemaVersion");
  const schemaVersion = rawVersion ? Number.parseInt(rawVersion, 10) : null;
  return {
    schemaVersion: Number.isFinite(schemaVersion) ? schemaVersion : null,
    appVersionName: values.get("appVersionName") || null,
  };
}

function readLocations(rows: ErpBackupSheetRows): Map<number, ErpBackupLocation> {
  const table = new HeaderTable(rows);
  const result = new Map<number, ErpBackupLocation>();
  if (!table.valid) return result;
  for (const row of table.dataRows()) {
    const id = table.int(row, "id");
    const code = table.str(row, "code");
    if (id === null || !code) continue;
    result.set(id, { code, displayName: table.str(row, "displayName") });
  }
  return result;
}

function readComponents(rows: ErpBackupSheetRows): Map<number, ErpBackupComponent> {
  const table = new HeaderTable(rows);
  const result = new Map<number, ErpBackupComponent>();
  if (!table.valid) return result;
  for (const row of table.dataRows()) {
    const id = table.int(row, "id");
    const rawPart = table.str(row, "partNumber");
    if (id === null || !rawPart) continue;
    const partNumber = rawPart.toUpperCase();
    result.set(id, {
      id,
      partNumber,
      name: table.str(row, "name"),
      brand: table.str(row, "brand"),
      packageName: table.str(row, "packageName"),
      category: table.str(row, "category"),
      description: table.str(row, "description"),
      isManual: MANUAL_PART_PATTERN.test(partNumber),
    });
  }
  return result;
}

interface InventoryRow {
  componentId: number;
  locationId: number;
  quantity: number;
}

function readInventoryRows(rows: ErpBackupSheetRows): InventoryRow[] {
  const table = new HeaderTable(rows);
  if (!table.valid) return [];
  const result: InventoryRow[] = [];
  for (const row of table.dataRows()) {
    const componentId = table.int(row, "componentId");
    const locationId = table.int(row, "locationId");
    const quantity = table.int(row, "quantity");
    if (componentId === null || quantity === null) continue;
    result.push({ componentId, locationId: locationId ?? -1, quantity });
  }
  return result;
}

/** 按表头名取列的表；数值单元格为 double，整数字段用四舍五入解析。 */
class HeaderTable {
  private readonly headers = new Map<string, number>();
  private readonly rows: ErpBackupSheetRows;
  readonly valid: boolean;

  constructor(rows: ErpBackupSheetRows) {
    this.rows = rows;
    const header = rows[0];
    if (!header) {
      this.valid = false;
      return;
    }
    header.forEach((cell, index) => {
      const name = cellText(cell).trim();
      if (name && !this.headers.has(name)) this.headers.set(name, index);
    });
    this.valid = this.headers.size > 0;
  }

  dataRows(): ErpBackupSheetRows {
    return this.rows.slice(1);
  }

  str(row: ErpBackupSheetRows[number], header: string): string | null {
    const index = this.headers.get(header);
    if (index === undefined) return null;
    const text = cellText(row[index]).trim();
    return text || null;
  }

  int(row: ErpBackupSheetRows[number], header: string): number | null {
    const index = this.headers.get(header);
    if (index === undefined) return null;
    const cell = row[index];
    if (cell === null || cell === undefined || cell === "") return null;
    const value = typeof cell === "number" ? cell : Number(cellText(cell).trim());
    return Number.isFinite(value) ? Math.round(value) : null;
  }
}

function cellText(cell: string | number | null | undefined): string {
  if (cell === null || cell === undefined) return "";
  return String(cell);
}
