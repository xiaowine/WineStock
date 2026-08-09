// 本文件是类型卫生扫描：禁止前端源码出现显式 any 类型，并统计 unknown 用量供趋势观察。
// 失败即 `pnpm test:type-hygiene` 失败。规范与信任边界清单见 frontend/docs/type-safety.md。
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import assert from "node:assert/strict";

const TESTS_ROOT = fileURLToPath(new URL(".", import.meta.url));
const SRC_ROOT = join(TESTS_ROOT, "..", "src");

/** 扫描豁免目录：语言资源与生成契约（生成物由 core OpenAPI 翻译而来，禁止手改）。 */
const EXCLUDED_SUFFIXES = [join("i18n", "locales"), join("api", "generated")].map((suffix) =>
  suffix.replace(/\\/g, "/"),
);

/** 显式 any 类型位置的正则集合；命中任一即失败。 */
const ANY_TYPE_PATTERNS = [
  /:\s*any\b/, // `: any` 注解
  /\bas\s+any\b/, // `as any` 断言
  /\bany\s*\[\]/, // `any[]`
  /\bany\s*>/, // `Foo<any>` 等泛型参数
];

/** 递归收集 .ts/.vue 源码文件（排除豁免目录）。 */
async function collectSources(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectSources(full)));
    } else if (/\.(ts|vue)$/.test(entry.name)) {
      const rel = full.replace(/\\/g, "/");
      if (!EXCLUDED_SUFFIXES.some((suffix) => rel.includes(suffix))) files.push(full);
    }
  }
  return files;
}

/**
 * 去掉注释与字符串/模板字面量后返回可匹配的源码，避免把文案、注释或 Vue 模板属性误判为类型。
 * 逐字符状态机：跟踪行/块/HTML 注释与单引号、双引号、模板字符串。
 */
function stripCommentsAndStrings(source) {
  let out = "";
  let i = 0;
  const n = source.length;
  while (i < n) {
    const c = source[i];
    const next = source[i + 1];
    if (c === "/" && next === "/") {
      while (i < n && source[i] !== "\n") i++;
    } else if (c === "/" && next === "*") {
      i += 2;
      while (i < n && !(source[i] === "*" && source[i + 1] === "/")) i++;
      i += 2;
    } else if (c === "<" && next === "!") {
      // HTML 注释（.vue 模板区）。
      const end = source.indexOf("-->", i);
      i = end === -1 ? n : end + 3;
    } else if (c === '"' || c === "'" || c === "`") {
      const quote = c;
      i++;
      while (i < n) {
        if (source[i] === "\\") i += 2;
        else if (source[i] === quote) {
          i++;
          break;
        } else i++;
      }
    } else {
      out += c;
      i++;
    }
  }
  return out;
}

test("源码不允许显式 any 类型", async () => {
  const files = await collectSources(SRC_ROOT);
  const hits = [];
  for (const file of files) {
    const stripped = stripCommentsAndStrings(await readFile(file, "utf8"));
    for (const pattern of ANY_TYPE_PATTERNS) {
      const match = stripped.match(pattern);
      if (match) hits.push({ file, pattern: pattern.source, match: match[0] });
    }
  }
  assert.deepEqual(
    hits,
    [],
    `发现显式 any 类型：\n${hits
      .map((hit) => `  ${hit.file} [${hit.pattern}] 命中 ${hit.match}`)
      .join("\n")}`,
  );
});

test("unknown 用量统计（只报告不拦截）", async () => {
  const files = await collectSources(SRC_ROOT);
  let count = 0;
  for (const file of files) {
    const source = await readFile(file, "utf8");
    count += (source.match(/\bunknown\b/g) ?? []).length;
  }
  // unknown 在信任边界是合法模式（必须配运行时守卫），数量只作趋势参考，不因变化失败。
  console.log(`[type-hygiene] unknown 词频：${count} 次 / ${files.length} 个文件`);
});
