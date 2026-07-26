// 本文件拥有立创商城订单导出表格的解析纯逻辑（行数组 → 订单号与四字段明细）；
// 它不读取文件、不依赖 SheetJS，也不执行物品匹配或草稿写入。

/** 工作表的原始单元格行；SheetJS `sheet_to_json(header: 1)` 的输出形状。 */
export type LcscOrderSheetRows = (string | number | null)[][];

/** 单条可导入明细；四字段之外的信息由调用方按 C 号查询补齐。 */
export interface LcscOrderImportLine {
  /** 明细在导出表格中的序号列取值，用于预览定位。 */
  rowLabel: string;
  /** 立创商品编号，已归一化为大写 C 前缀。 */
  productCode: string;
  /** 订购数量；剥离“个”等单位后缀后的正数。 */
  quantity: number;
  /** 实际成交单价；剥离 ￥ 前缀与 /个 后缀后的非负数。 */
  unitPrice: number;
}

/** 被排除的明细行及原因，供预览呈现。 */
export interface LcscOrderSkippedLine {
  rowLabel: string;
  productCode: string | null;
  reason: string;
}

export type LcscOrderParseResult =
  | {
      ok: true;
      /** 订单编号（SO 开头）；导出异常缺失时为 null，调用方不填写来源。 */
      orderNo: string | null;
      lines: LcscOrderImportLine[];
      skipped: LcscOrderSkippedLine[];
    }
  | { ok: false; error: string };

const ORDER_NO_PATTERN = /^SO\d+$/i;
const PRODUCT_CODE_PATTERN = /^C\d+$/i;
const HEADER_PRODUCT_CODE = "商品编号";
const HEADER_QUANTITY = "订购数量（修改后）";
const HEADER_UNIT_PRICE = "商品单价";
const HEADER_NOT_SHIPPED = "是否不发此货";
const DETAIL_MARKER = "商品明细列表";

/**
 * 解析立创订单导出工作簿。
 * 导出文件固定包含真数据表与一张全零模板表（订单编号为 0），按“订单编号形如 SO+数字”
 * 或“存在合法 C 号明细”识别真表；明细定位不依赖固定行号，按「商品明细列表」标记与表头名映射。
 */
export function parseLcscOrderSheets(
  sheets: { name: string; rows: LcscOrderSheetRows }[],
): LcscOrderParseResult {
  let fallback: LcscOrderParseResult | null = null;
  for (const sheet of sheets) {
    const parsed = parseSheet(sheet.rows);
    if (!parsed) continue;
    if (parsed.orderNo !== null && parsed.lines.length > 0) return parsed;
    // 记录第一张能解析出明细的表作为兜底（如订单号异常缺失时仍可导入明细）。
    if (!fallback && parsed.lines.length > 0) fallback = parsed;
  }
  return (
    fallback ?? {
      ok: false,
      error: "未在文件中找到立创订单明细，请确认选择的是立创商城「订单详情」导出的表格。",
    }
  );
}

function parseSheet(rows: LcscOrderSheetRows): Extract<LcscOrderParseResult, { ok: true }> | null {
  const markerIndex = rows.findIndex((row) => cellText(row[0]).includes(DETAIL_MARKER));
  if (markerIndex < 0 || markerIndex + 1 >= rows.length) return null;
  const header = rows[markerIndex + 1];
  const columnOf = (name: string) => header.findIndex((cell) => cellText(cell).trim() === name);
  const productCodeColumn = columnOf(HEADER_PRODUCT_CODE);
  const quantityColumn = columnOf(HEADER_QUANTITY);
  const unitPriceColumn = columnOf(HEADER_UNIT_PRICE);
  const notShippedColumn = columnOf(HEADER_NOT_SHIPPED);
  if (productCodeColumn < 0 || quantityColumn < 0 || unitPriceColumn < 0) return null;

  const orderNoRow = rows.find((row) => cellText(row[0]).includes("订单编号"));
  const orderNoValue = cellText(orderNoRow?.[1]).trim().toUpperCase();
  const orderNo = ORDER_NO_PATTERN.test(orderNoValue) ? orderNoValue : null;

  const lines: LcscOrderImportLine[] = [];
  const skipped: LcscOrderSkippedLine[] = [];
  for (const row of rows.slice(markerIndex + 2)) {
    const rawCode = cellText(row[productCodeColumn]).trim();
    if (!rawCode) continue;
    const rowLabel = cellText(row[0]).trim() || String(lines.length + skipped.length + 1);
    if (!PRODUCT_CODE_PATTERN.test(rawCode)) {
      skipped.push({ rowLabel, productCode: null, reason: `商品编号「${rawCode}」不是合法 C 号` });
      continue;
    }
    const productCode = `C${rawCode.slice(1)}`;
    if (notShippedColumn >= 0 && cellText(row[notShippedColumn]).trim()) {
      skipped.push({ rowLabel, productCode, reason: "该行标记为不发货" });
      continue;
    }
    const quantity = parseQuantity(row[quantityColumn]);
    if (quantity === null) {
      skipped.push({
        rowLabel,
        productCode,
        reason: `订购数量「${cellText(row[quantityColumn])}」无法解析`,
      });
      continue;
    }
    const unitPrice = parseUnitPrice(row[unitPriceColumn]);
    if (unitPrice === null) {
      skipped.push({
        rowLabel,
        productCode,
        reason: `商品单价「${cellText(row[unitPriceColumn])}」无法解析`,
      });
      continue;
    }
    lines.push({ rowLabel, productCode, quantity, unitPrice });
  }

  if (lines.length === 0 && skipped.length === 0) return null;
  return { ok: true, orderNo, lines, skipped };
}

/** 解析“300个”式数量：取前导数字并要求为正数。 */
function parseQuantity(cell: string | number | null): number | null {
  if (typeof cell === "number") return Number.isFinite(cell) && cell > 0 ? cell : null;
  const matched = /^([0-9]+(?:\.[0-9]+)?)/.exec(cellText(cell).trim());
  if (!matched) return null;
  const value = Number(matched[1]);
  return Number.isFinite(value) && value > 0 ? value : null;
}

/** 解析“￥0.056620/个”式单价：取其中的数字并要求非负。 */
function parseUnitPrice(cell: string | number | null): number | null {
  if (typeof cell === "number") return Number.isFinite(cell) && cell >= 0 ? cell : null;
  const matched = /([0-9]+(?:\.[0-9]+)?)/.exec(cellText(cell));
  if (!matched) return null;
  const value = Number(matched[1]);
  return Number.isFinite(value) && value >= 0 ? value : null;
}

function cellText(cell: string | number | null | undefined): string {
  if (cell === null || cell === undefined) return "";
  return String(cell);
}
