import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/lcsc/bagCode.ts", import.meta.url);
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
const { parseLcscBagCode } = await import(moduleUrl);

test("parses real bag samples with order, part and quantity", () => {
  assert.deepEqual(
    parseLcscBagCode(
      "{on:SO26010926692,pc:C2687125,pm:SM05.TCT,qty:100,mc:,cc:1,pdi:193942893,hp:11}",
    ),
    {
      orderNo: "SO26010926692",
      productCode: "C2687125",
      manufacturerPart: "SM05.TCT",
      quantity: 100,
    },
  );
  assert.deepEqual(
    parseLcscBagCode(
      "{on:SO26010926692,pc:C21882319,pm:GT-F0509SR15-08SMT01,qty:85,mc:,cc:1,pdi:193942891,hp:null}",
    ),
    {
      orderNo: "SO26010926692",
      productCode: "C21882319",
      manufacturerPart: "GT-F0509SR15-08SMT01",
      quantity: 85,
    },
  );
});

test("normalizes lowercase product code and tolerates spaces", () => {
  const parsed = parseLcscBagCode("{ on : SO1 , pc : c22356631 , qty : 1000 }");
  assert.equal(parsed?.productCode, "C22356631");
  assert.equal(parsed?.quantity, 1000);
  assert.equal(parsed?.orderNo, "SO1");
});

test("missing or placeholder fields become null while pc keeps the result valid", () => {
  const parsed = parseLcscBagCode("{pc:C123,pm:null,qty:,on:}");
  assert.deepEqual(parsed, {
    orderNo: null,
    productCode: "C123",
    manufacturerPart: null,
    quantity: null,
  });
});

test("invalid quantity values degrade to null instead of rejecting", () => {
  assert.equal(parseLcscBagCode("{pc:C123,qty:0}")?.quantity, null);
  assert.equal(parseLcscBagCode("{pc:C123,qty:12.5}")?.quantity, null);
  assert.equal(parseLcscBagCode("{pc:C123,qty:abc}")?.quantity, null);
});

test("rejects content that is not an LCSC bag code", () => {
  assert.equal(parseLcscBagCode(""), null);
  assert.equal(parseLcscBagCode("https://example.com/qr"), null);
  assert.equal(parseLcscBagCode("C2687125"), null);
  assert.equal(parseLcscBagCode("{on:SO1,qty:100}"), null, "缺少 pc");
  assert.equal(parseLcscBagCode("{pc:2687125}"), null, "缺少 C 前缀");
  assert.equal(parseLcscBagCode("{pc:CX123}"), null, "C 后必须全数字");
  assert.equal(parseLcscBagCode('{"pc":"C123"}'), null, "带引号的 JSON 不是料袋格式");
  assert.equal(parseLcscBagCode("{pc:C123,broken}"), null, "存在无冒号分段");
  assert.equal(parseLcscBagCode(`{pc:C123,${"x".repeat(600)}:1}`), null, "超长输入");
});

test("duplicate keys keep the first occurrence", () => {
  assert.equal(parseLcscBagCode("{pc:C111,pc:C222}")?.productCode, "C111");
});
