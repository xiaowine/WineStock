import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/clipboard/model.ts", import.meta.url);
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
const { COPY_DETAIL_MAX_LENGTH, copyNoticeDetail, normalizeCopyableValue } = await import(
  moduleUrl
);

test("string binding trims and rejects empty", () => {
  assert.deepEqual(normalizeCopyableValue(" C123 ", ""), { text: "C123" });
  assert.equal(normalizeCopyableValue("   ", "fallback"), null);
});

test("object binding keeps label and validates text", () => {
  assert.deepEqual(normalizeCopyableValue({ text: "http://a", label: "连接地址" }, ""), {
    text: "http://a",
    label: "连接地址",
  });
  assert.deepEqual(normalizeCopyableValue({ text: " x ", label: "" }, ""), { text: "x" });
  assert.equal(normalizeCopyableValue({ text: 42 }, ""), null);
  assert.equal(normalizeCopyableValue({ text: "  " }, ""), null);
});

test("missing binding falls back to element text", () => {
  assert.deepEqual(normalizeCopyableValue(undefined, "  SO123  "), { text: "SO123" });
  assert.equal(normalizeCopyableValue(null, "   "), null);
});

test("unsupported binding types are rejected", () => {
  assert.equal(normalizeCopyableValue(42, "fallback"), null);
  assert.equal(normalizeCopyableValue(["a"], "fallback"), null);
});

test("notice detail hides long content such as JSON dumps", () => {
  const short = "x".repeat(COPY_DETAIL_MAX_LENGTH);
  assert.equal(copyNoticeDetail(short), short);
  assert.equal(copyNoticeDetail(short + "x"), undefined);
});
