import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/erp/backupImport.ts", import.meta.url);
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
const { parseErpBackupSheets } = await import(moduleUrl);

function sheets(overrides = {}) {
  return [
    {
      name: "meta",
      rows: overrides.meta ?? [
        ["schemaVersion", "1"],
        ["appVersionName", "1.5.0"],
      ],
    },
    {
      name: "storage_locations",
      rows: overrides.locations ?? [
        ["id", "code", "displayName"],
        [1, "A1", "A1"],
        [2, "C3", null],
        [3, "EMPTY", "空库位"],
      ],
    },
    {
      name: "components",
      rows: overrides.components ?? [
        ["id", "partNumber", "name", "brand", "packageName", "category", "description"],
        [1, "c21882319", "GT-F0509", "G-Switch", "SMD", "FFC/FPC连接器", "前掀盖下接触"],
        [2, "C2687125", "SM05.TCT", "UMW", "SOT-23", "TVS/ESD", "ESD 保护"],
        [3, "C01", "手工件", null, null, null, null],
      ],
    },
    {
      name: "inventory_items",
      rows: overrides.inventory ?? [
        ["id", "componentId", "locationId", "quantity"],
        [1, 1, 1, 85],
        [2, 2, 2, 100],
        [3, 3, 1, 5],
      ],
    },
  ];
}

test("parses four tables, joins and normalizes part numbers", () => {
  const result = parseErpBackupSheets(sheets());
  assert.equal(result.ok, true);
  assert.equal(result.appVersion, "1.5.0");
  // 真 C 码器件按库存 join；partNumber 大写归一。
  const codes = result.items.map((item) => item.component.partNumber).sort();
  assert.deepEqual(codes, ["C21882319", "C2687125"]);
  const first = result.items.find((item) => item.component.partNumber === "C21882319");
  assert.equal(first.quantity, 85);
  assert.equal(first.locationCode, "A1");
  assert.equal(first.component.category, "FFC/FPC连接器");
  // 未被库存引用的空库位也属于备份结构，必须保留并在导入阶段落地。
  assert.deepEqual(result.locations.map((location) => location.code).sort(), ["A1", "C3", "EMPTY"]);
});

test("splits C0 manual parts into skipped with accumulated quantity", () => {
  const result = parseErpBackupSheets(
    sheets({
      inventory: [
        ["id", "componentId", "locationId", "quantity"],
        [1, 1, 1, 85],
        [2, 3, 1, 5],
        [3, 3, 2, 7],
      ],
    }),
  );
  assert.equal(result.ok, true);
  assert.deepEqual(
    result.items.map((item) => item.component.partNumber),
    ["C21882319"],
  );
  assert.deepEqual(result.skippedManual, [{ partNumber: "C01", quantity: 12 }]);
});

test("rounds double quantities and drops non-positive rows", () => {
  const result = parseErpBackupSheets(
    sheets({
      inventory: [
        ["id", "componentId", "locationId", "quantity"],
        [1, 1, 1, 84.9999],
        [2, 2, 2, 0],
        [3, 3, 1, -3],
      ],
    }),
  );
  assert.equal(result.ok, true);
  assert.deepEqual(
    result.items.map((item) => item.quantity),
    [85],
  );
  assert.deepEqual(result.skippedManual, []);
});

test("drops orphan inventory rows and nulls unmatched location", () => {
  const result = parseErpBackupSheets(
    sheets({
      inventory: [
        ["id", "componentId", "locationId", "quantity"],
        [1, 1, 99, 10], // 库位 99 不存在 → locationCode null
        [2, 99, 1, 10], // 器件 99 不存在 → 整行剔除
      ],
    }),
  );
  assert.equal(result.ok, true);
  assert.equal(result.items.length, 1);
  assert.equal(result.items[0].locationCode, null);
});

test("is insensitive to column order via header names", () => {
  const result = parseErpBackupSheets(
    sheets({
      components: [
        ["partNumber", "id", "category", "name"],
        ["C21882319", 1, "连接器", "GT"],
      ],
      inventory: [
        ["quantity", "locationId", "componentId", "id"],
        [42, 1, 1, 1],
      ],
    }),
  );
  assert.equal(result.ok, true);
  assert.equal(result.items[0].quantity, 42);
  assert.equal(result.items[0].component.name, "GT");
});

test("rejects unsupported schema version", () => {
  const result = parseErpBackupSheets(sheets({ meta: [["schemaVersion", "2"]] }));
  assert.equal(result.ok, false);
});

test("rejects backup missing a required table", () => {
  const withoutComponents = sheets().filter((sheet) => sheet.name !== "components");
  const result = parseErpBackupSheets(withoutComponents);
  assert.equal(result.ok, false);
});
