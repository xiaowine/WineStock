// 本文件是 i18n 防回退扫描：检查语言资源文件以外的前端源码是否存在硬编码中文文案，
// 并核对 core/shared 错误码在语言包中的键覆盖。失败即 `pnpm test:i18n-scan` 失败。
import { readFile, readdir } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import assert from "node:assert/strict";

const TESTS_ROOT = fileURLToPath(new URL(".", import.meta.url));
const FRONTEND_ROOT = join(TESTS_ROOT, "..");
const REPO_ROOT = join(FRONTEND_ROOT, "..");
const SRC_ROOT = join(FRONTEND_ROOT, "src");

/** 扫描豁免目录：语言资源与生成契约。 */
const EXCLUDED_SUFFIXES = [join("i18n", "locales"), join("api", "generated")].map((suffix) =>
  suffix.replace(/\\/g, "/"),
);

/** 递归收集 .vue/.ts 文件（排除豁免目录与测试自身）。 */
async function collectSources(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (EXCLUDED_SUFFIXES.some((suffix) => path.replace(/\\/g, "/").includes(suffix))) {
      continue;
    }
    if (entry.isDirectory()) {
      files.push(...(await collectSources(path)));
    } else if (entry.name.endsWith(".vue") || entry.name.endsWith(".ts")) {
      files.push(path);
    }
  }
  return files;
}

/**
 * 去掉注释后检查文本中是否残留中文。
 * 逐字符状态机：跟踪字符串/模板字面量与行/块/HTML 注释，避免把 URL 或注释中的中文误判；
 * 模板字面量整体按文案区域处理（含 ${} 插值表达式中的中文——那本身也属应迁移的文案）。
 */
function hasHardcodedChinese(source) {
  let inLineComment = false;
  let inBlockComment = false;
  let inSingleQuote = false;
  let inDoubleQuote = false;
  let inTemplateLiteral = false;
  let result = "";
  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];
    const next = source[index + 1];
    if (inLineComment) {
      if (char === "\n") {
        inLineComment = false;
        result += char;
      }
      continue;
    }
    if (inBlockComment) {
      if (char === "*" && next === "/") {
        inBlockComment = false;
        index += 1;
      }
      continue;
    }
    if (!inSingleQuote && !inDoubleQuote && !inTemplateLiteral) {
      if (char === "/" && next === "/") {
        inLineComment = true;
        index += 1;
        continue;
      }
      if (char === "/" && next === "*") {
        inBlockComment = true;
        index += 1;
        continue;
      }
      // Vue 模板的 HTML 注释。
      if (char === "<" && source.startsWith("<!--", index)) {
        const end = source.indexOf("-->", index + 4);
        index = end >= 0 ? end + 2 : source.length;
        continue;
      }
      if (char === "'") {
        inSingleQuote = true;
        result += char;
        continue;
      }
      if (char === '"') {
        inDoubleQuote = true;
        result += char;
        continue;
      }
      if (char === "`") {
        inTemplateLiteral = true;
        result += char;
        continue;
      }
      result += char;
      continue;
    }
    // 字符串/模板字面量内部（按连续反斜杠奇偶判定是否真正转义）。
    result += char;
    const escaped = countPrecedingBackslashes(source, index) % 2 === 1;
    if (inSingleQuote && char === "'" && !escaped) inSingleQuote = false;
    if (inDoubleQuote && char === '"' && !escaped) inDoubleQuote = false;
    if (inTemplateLiteral && char === "`" && !escaped) inTemplateLiteral = false;
  }
  return /[\u4e00-\u9fff]/.test(result);
}

/** 统计 index 之前连续的 `\` 数量，用于转义判定。 */
function countPrecedingBackslashes(source, index) {
  let count = 0;
  for (let cursor = index - 1; cursor >= 0 && source[cursor] === "\\"; cursor -= 1) {
    count += 1;
  }
  return count;
}

test("源码（语言资源之外）不允许硬编码中文文案", async () => {
  const files = await collectSources(SRC_ROOT);
  const violations = [];
  for (const file of files) {
    const source = await readFile(file, "utf8");
    if (hasHardcodedChinese(source)) {
      violations.push(relative(FRONTEND_ROOT, file));
    }
  }
  assert.deepEqual(violations, [], "以下文件仍含硬编码中文文案，请迁移到 i18n 语言包");
});

/** 从 Rust 源码中提取 `"..."` 字符串字面量。 */
function extractRustStringLiterals(source) {
  const matches = [...source.matchAll(/"([^"\\]*(?:\\.[^"\\]*)*)"/g)];
  return matches.map((match) => match[1]);
}

/** 读取 core 错误码注册表与 shared garde 码清单，核对语言包键覆盖。 */
async function assertCodeCoverage() {
  const coreCodesPath = join(
    REPO_ROOT,
    "core",
    "src",
    "http",
    "error_codes.rs",
  );
  const coreCodesSource = await readFile(coreCodesPath, "utf8");
  const registryBody = coreCodesSource.match(/API_ERROR_CODES:\s*&\[([\s\S]*?)\];/);
  assert.ok(registryBody, "core 错误码注册表必须存在");
  const coreCodes = extractRustStringLiterals(registryBody[1]);

  const sharedCodesSource = await readFile(
    join(REPO_ROOT, "shared", "src", "garde_code.rs"),
    "utf8",
  );
  // BUILTIN_RULES 每行是 (消息模板, 稳定码)，只取第二个元素。
  const builtinBody = sharedCodesSource.match(/BUILTIN_RULES:\s*&\[([\s\S]*?)\];/);
  assert.ok(builtinBody, "shared garde 内置码映射表必须存在");
  const builtinCodes = [
    ...builtinBody[1].matchAll(/\("([^"]+)",\s*"([^"]+)"\)/g),
  ].map((match) => match[2]);
  const passthroughBody = sharedCodesSource.match(/match message \{([\s\S]*?)\n    \}/);
  const passthroughCodes = extractRustStringLiterals(passthroughBody?.[1] ?? "");

  const errorPack = await readFile(join(SRC_ROOT, "i18n", "locales", "zh-CN", "error.ts"), "utf8");
  const validationPack = await readFile(
    join(SRC_ROOT, "i18n", "locales", "zh-CN", "validation.ts"),
    "utf8",
  );

  const missingErrorCodes = coreCodes.filter((code) => !errorPack.includes(`${code}:`));
  const missingValidationCodes = [
    ...new Set([...builtinCodes, ...passthroughCodes]),
  ].filter((code) => !validationPack.includes(`${code}:`));
  assert.deepEqual(
    missingErrorCodes,
    [],
    "core 注册的错误码缺少 error.* 语言包键（error.ts）",
  );
  assert.deepEqual(
    missingValidationCodes,
    [],
    "shared 产出的校验码缺少 validation.* 语言包键（validation.ts）",
  );
}

test("core/shared 稳定码在语言包中必须有键覆盖", assertCodeCoverage);
