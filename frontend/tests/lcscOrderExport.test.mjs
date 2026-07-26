import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/lcsc/orderExport.ts", import.meta.url);
const source = await readFile(sourceUrl, "utf8");
const transpiled = ts.transpileModule(source, {
  fileName: sourceUrl.pathname,
  compilerOptions: {
    module: ts.ModuleKind.ES2022,
    target: ts.ScriptTarget.ES2022,
    verbatimModuleSyntax: true,
  },
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`;
const { parseLcscOrderSheets } = await import(moduleUrl);

// 与真实导出一致的表头（注意「封装 」自带尾部空格）。
const HEADER = [
  "序号",
  "商品编号",
  "品牌",
  "厂家型号",
  "封装 ",
  "商品名称",
  "订购数量（修改后）",
  "是否不发此货",
  "毛重（kg）",
  "商品单价",
  "商品金额",
];

function sheetOf(orderNo, itemRows, { name = "立创商城订单详情" } = {}) {
  return {
    name,
    rows: [
      ["订单编号：", orderNo],
      ["下单时间：", "2026-07-09 20:30:00"],
      ["商品明细列表"],
      HEADER,
      ...itemRows,
    ],
  };
}

test("parses a realistic sheet into order number and four-field lines", () => {
  const result = parseLcscOrderSheets([
    sheetOf("SO26070920712", [
      [
        "1",
        "C51953485",
        "CCX(晁禾)",
        "B5819WS",
        "SOD-323",
        "肖特基",
        "300个",
        null,
        "0.0000283",
        "￥0.056620/个",
        "￥16.99",
      ],
      [
        "2",
        "C2687125",
        "TI",
        "SM05.TCT",
        "SOT-23",
        "ESD",
        "100个",
        null,
        "0.00002",
        "￥0.128000/个",
        "￥12.80",
      ],
    ]),
  ]);
  assert.equal(result.ok, true);
  assert.equal(result.orderNo, "SO26070920712");
  assert.deepEqual(result.lines, [
    { rowLabel: "1", productCode: "C51953485", quantity: 300, unitPrice: 0.05662 },
    { rowLabel: "2", productCode: "C2687125", quantity: 100, unitPrice: 0.128 },
  ]);
  assert.deepEqual(result.skipped, []);
});

test("rejects the all-zero template sheet and picks the real one", () => {
  const template = sheetOf(0, [["0", "0", "0", "0", "0", "0", "0", null, "0", "0", "0"]], {
    name: "Sheet1",
  });
  const real = sheetOf("SO2601090825", [
    ["1", "C123", "b", "m", "p", "n", "50个", null, "0", "￥1.000000/个", "￥50.00"],
  ]);
  const result = parseLcscOrderSheets([template, real]);
  assert.equal(result.ok, true);
  assert.equal(result.orderNo, "SO2601090825");
  assert.equal(result.lines.length, 1);
});

test("skips not-shipped rows and unparsable quantity or price with reasons", () => {
  const result = parseLcscOrderSheets([
    sheetOf("SO1", [
      ["1", "C1", "b", "m", "p", "n", "10个", "是", "0", "￥1.000000/个", "￥10.00"],
      ["2", "C2", "b", "m", "p", "n", "无货", null, "0", "￥1.000000/个", "￥0.00"],
      ["3", "C3", "b", "m", "p", "n", "10个", null, "0", "面议", "￥0.00"],
      ["4", "C4", "b", "m", "p", "n", "10个", null, "0", "￥2.500000/个", "￥25.00"],
    ]),
  ]);
  assert.equal(result.ok, true);
  assert.deepEqual(
    result.skipped.map((line) => [line.rowLabel, line.productCode]),
    [
      ["1", "C1"],
      ["2", "C2"],
      ["3", "C3"],
    ],
  );
  assert.deepEqual(result.lines, [
    { rowLabel: "4", productCode: "C4", quantity: 10, unitPrice: 2.5 },
  ]);
});

test("normalizes lowercase product code and rejects non C-number codes", () => {
  const result = parseLcscOrderSheets([
    sheetOf("SO1", [
      ["1", "c987654", "b", "m", "p", "n", "5个", null, "0", "￥0.100000/个", "￥0.50"],
      ["2", "X123", "b", "m", "p", "n", "5个", null, "0", "￥0.100000/个", "￥0.50"],
    ]),
  ]);
  assert.equal(result.lines[0].productCode, "C987654");
  assert.equal(result.skipped.length, 1);
  assert.equal(result.skipped[0].productCode, null);
});

test("keeps lines with a null order number when the label row is malformed", () => {
  const sheet = sheetOf("SO1", [
    ["1", "C1", "b", "m", "p", "n", "5个", null, "0", "￥0.100000/个", "￥0.50"],
  ]);
  sheet.rows[0] = ["订单编号：", null];
  const result = parseLcscOrderSheets([sheet]);
  assert.equal(result.ok, true);
  assert.equal(result.orderNo, null);
  assert.equal(result.lines.length, 1);
});

test("numeric cells for quantity and price are accepted as-is", () => {
  const result = parseLcscOrderSheets([
    sheetOf("SO1", [["1", "C1", "b", "m", "p", "n", 25, null, "0", 0.3456, "￥8.64"]]),
  ]);
  assert.deepEqual(result.lines, [
    { rowLabel: "1", productCode: "C1", quantity: 25, unitPrice: 0.3456 },
  ]);
});

test("returns a friendly error when no sheet contains the detail marker", () => {
  const result = parseLcscOrderSheets([
    { name: "Sheet1", rows: [["随便", "内容"]] },
    { name: "Sheet2", rows: [] },
  ]);
  assert.equal(result.ok, false);
  assert.match(result.error, /立创/);
});

// ---- 本机样本用例：目录里有 .xls 时对真实导出执行断言，否则跳过。 ----

const fixturesUrl = new URL("./fixtures/lcsc-orders/", import.meta.url);
const fixtureFiles = (await readdir(fixturesUrl).catch(() => [])).filter((name) =>
  /\.xlsx?$/i.test(name),
);

test(
  "parses local real export samples",
  { skip: fixtureFiles.length === 0 && "目录中没有样本文件" },
  async () => {
    const XLSX = await import("xlsx");
    for (const name of fixtureFiles) {
      const workbook = XLSX.read(await readFile(new URL(name, fixturesUrl)), { type: "buffer" });
      const sheets = workbook.SheetNames.map((sheetName) => {
        const sheet = workbook.Sheets[sheetName];
        const rows = sheet["!ref"]
          ? XLSX.utils.sheet_to_json(sheet, { header: 1, defval: null, blankrows: false })
          : [];
        return { name: sheetName, rows };
      });
      const result = parseLcscOrderSheets(sheets);
      assert.equal(result.ok, true, `${name} 应能解析`);
      assert.match(result.orderNo ?? "", /^SO\d+$/, `${name} 的订单号`);
      assert.ok(result.lines.length > 0, `${name} 应有明细行`);
      for (const line of result.lines) {
        assert.match(line.productCode, /^C\d+$/, `${name} 行 ${line.rowLabel} 的 C 号`);
        assert.ok(
          Number.isFinite(line.quantity) && line.quantity > 0,
          `${name} 行 ${line.rowLabel} 的数量`,
        );
        assert.ok(
          Number.isFinite(line.unitPrice) && line.unitPrice >= 0,
          `${name} 行 ${line.rowLabel} 的单价`,
        );
      }
    }
  },
);
