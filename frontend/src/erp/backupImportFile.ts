// 本文件拥有第三方 ERP 备份文件的读取入口：动态加载 SheetJS，把 .xlsx 各工作表
// 转成行数组后交给 backupImport.ts 的纯解析逻辑；不做库位/物品创建与草稿写入。
// 解析失败在此唯一入口记排查事件，便于第三方改备份格式时第一时间发现。
import { trackTelemetryIssue } from "../telemetry/clarity";
import {
  parseErpBackupSheets,
  type ErpBackupParseResult,
  type ErpBackupSheetRows,
} from "./backupImport";

/**
 * 解析用户选择的 ERP 备份文件。
 * SheetJS 体积较大，这里按需动态加载，避免进入主包（与订单导入同一策略）。
 */
export async function parseErpBackupFile(file: File): Promise<ErpBackupParseResult> {
  const result = await readAndParse(file);
  if (!result.ok) trackTelemetryIssue("erp_backup_parse_failed");
  return result;
}

async function readAndParse(file: File): Promise<ErpBackupParseResult> {
  let workbook: import("xlsx").WorkBook;
  try {
    const XLSX = await import("xlsx");
    workbook = XLSX.read(await file.arrayBuffer(), { type: "array" });
  } catch {
    return { ok: false, error: "文件读取失败，请确认选择的是 ERP 导出的 .xlsx 备份文件。" };
  }
  const { utils } = await import("xlsx");
  const sheets = workbook.SheetNames.map((name) => {
    const sheet = workbook.Sheets[name];
    const rows: ErpBackupSheetRows = sheet?.["!ref"]
      ? (utils.sheet_to_json(sheet, {
          header: 1,
          defval: null,
          blankrows: false,
        }) as ErpBackupSheetRows)
      : [];
    return { name, rows };
  });
  return parseErpBackupSheets(sheets);
}
