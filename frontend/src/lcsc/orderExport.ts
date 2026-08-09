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
  /** 消息键；由展示层翻译后展示。 */
  reason: string;
  /** 消息键的插值参数；无参数时省略。 */
  params?: Record<string, string>;
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
// 下列表头/标记为立创导出文件的数据常量（与 UI 语言无关），转义保持运行时值与导出格式一致。
const HEADER_PRODUCT_CODE = "\u5546\u54c1\u7f16\u53f7"; // 商品编号
const HEADER_QUANTITY = "\u8ba2\u8d2d\u6570\u91cf\uff08\u4fee\u6539\u540e\uff09"; // 订购数量（修改后）
const HEADER_UNIT_PRICE = "\u5546\u54c1\u5355\u4ef7"; // 商品单价
const HEADER_NOT_SHIPPED = "\u662f\u5426\u4e0d\u53d1\u6b64\u8d27"; // 是否不发此货
const DETAIL_MARKER = "\u5546\u54c1\u660e\u7ec6\u5217\u8868"; // 商品明细列表
const ORDER_NO_MARKER = "\u8ba2\u5355\u7f16\u53f7"; // 订单编号

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
      // 错误文案为消息键，由展示层 translateMessageOrNull 翻译后呈现。
      error: "stockDraft.lcscOrderDetailNotFound",
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

  const orderNoRow = rows.find((row) => cellText(row[0]).includes(ORDER_NO_MARKER));
  const orderNoValue = cellText(orderNoRow?.[1]).trim().toUpperCase();
  const orderNo = ORDER_NO_PATTERN.test(orderNoValue) ? orderNoValue : null;

  const lines: LcscOrderImportLine[] = [];
  const skipped: LcscOrderSkippedLine[] = [];
  for (const row of rows.slice(markerIndex + 2)) {
    const rawCode = cellText(row[productCodeColumn]).trim();
    if (!rawCode) continue;
    const rowLabel = cellText(row[0]).trim() || String(lines.length + skipped.length + 1);
    if (!PRODUCT_CODE_PATTERN.test(rawCode)) {
      // reason 为消息键，携带插值参数；由展示层翻译后呈现。
      skipped.push({
        rowLabel,
        productCode: null,
        reason: "stockDraft.invalidProductCode",
        params: { code: rawCode },
      });
      continue;
    }
    const productCode = `C${rawCode.slice(1)}`;
    if (notShippedColumn >= 0 && cellText(row[notShippedColumn]).trim()) {
      skipped.push({ rowLabel, productCode, reason: "stockDraft.rowMarkedNotShipped" });
      continue;
    }
    const quantity = parseQuantity(row[quantityColumn]);
    if (quantity === null) {
      skipped.push({
        rowLabel,
        productCode,
        reason: "stockDraft.quantityUnparsable",
        params: { value: cellText(row[quantityColumn]) },
      });
      continue;
    }
    const unitPrice = parseUnitPrice(row[unitPriceColumn]);
    if (unitPrice === null) {
      skipped.push({
        rowLabel,
        productCode,
        reason: "stockDraft.unitPriceUnparsable",
        params: { value: cellText(row[unitPriceColumn]) },
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
