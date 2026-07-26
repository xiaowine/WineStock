// 本文件拥有立创订单导出文件的读取入口：动态加载 SheetJS，把 .xls 工作簿
// 转成行数组后交给 orderExport.ts 的纯解析逻辑；不做物品匹配与草稿写入。
import {
  parseLcscOrderSheets,
  type LcscOrderParseResult,
  type LcscOrderSheetRows,
} from "./orderExport";

/**
 * 解析用户选择的立创订单导出文件。
 * SheetJS 体积较大，这里按需动态加载，避免进入主包。
 */
export async function parseLcscOrderFile(file: File): Promise<LcscOrderParseResult> {
  let workbook: import("xlsx").WorkBook;
  try {
    const XLSX = await import("xlsx");
    workbook = XLSX.read(await file.arrayBuffer(), { type: "array" });
  } catch {
    return { ok: false, error: "文件读取失败，请确认选择的是立创商城导出的 .xls 表格。" };
  }
  const { utils } = await import("xlsx");
  const sheets = workbook.SheetNames.map((name) => {
    const sheet = workbook.Sheets[name];
    const rows: LcscOrderSheetRows = sheet?.["!ref"]
      ? (utils.sheet_to_json(sheet, {
          header: 1,
          defval: null,
          blankrows: false,
        }) as LcscOrderSheetRows)
      : [];
    return { name, rows };
  });
  return parseLcscOrderSheets(sheets);
}
