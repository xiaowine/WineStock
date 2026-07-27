import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/theme/model.ts", import.meta.url);
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
const { parseThemePreference, resolveTheme, THEME_STORAGE_KEY } = await import(moduleUrl);

function readHexThemeMap(source, mapName) {
  const body = source.match(new RegExp(`\\$${mapName}: \\(([\\s\\S]*?)\\n\\);`))?.[1];
  assert.ok(body, `missing ${mapName} theme map`);
  return new Map(
    [...body.matchAll(/"([\w-]+)":\s*(#[0-9a-f]{6})/gi)].map((match) => [match[1], match[2]]),
  );
}

function relativeLuminance(hex) {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function contrastRatio(first, second) {
  const lighter = Math.max(relativeLuminance(first), relativeLuminance(second));
  const darker = Math.min(relativeLuminance(first), relativeLuminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

test("theme preference accepts only the three supported values", () => {
  assert.equal(parseThemePreference("system"), "system");
  assert.equal(parseThemePreference("light"), "light");
  assert.equal(parseThemePreference("dark"), "dark");
});

test("missing, damaged and future values fall back to system", () => {
  assert.equal(parseThemePreference(null), "system");
  assert.equal(parseThemePreference(""), "system");
  assert.equal(parseThemePreference("sepia"), "system");
  assert.equal(parseThemePreference({ value: "dark" }), "system");
});

test("system preference resolves from the current media query", () => {
  assert.equal(resolveTheme("system", false), "light");
  assert.equal(resolveTheme("system", true), "dark");
});

test("manual preferences override the current system theme", () => {
  assert.equal(resolveTheme("light", true), "light");
  assert.equal(resolveTheme("dark", false), "dark");
});

test("first-paint bootstrap uses the same versioned storage key", async () => {
  const html = await readFile(new URL("../index.html", import.meta.url), "utf8");
  assert.equal(THEME_STORAGE_KEY, "winestock.theme.preference.v1");
  assert.match(html, new RegExp(THEME_STORAGE_KEY.replaceAll(".", "\\.")));
});

test("light and dark semantic foregrounds keep readable contrast with their backgrounds", async () => {
  const tokens = await readFile(
    new URL("../src/styles/foundation/_tokens.scss", import.meta.url),
    "utf8",
  );
  const checks = [
    ["color-on-accent", "color-accent", 4.5],
    ["color-on-accent", "color-accent-strong", 4.5],
    ["color-on-danger", "color-danger", 4.5],
    ["color-accent-strong", "color-accent-soft", 4.5],
    ["color-success", "color-success-soft", 4.5],
    ["color-warn", "color-warn-soft", 4.5],
    ["color-danger", "color-danger-soft", 4.5],
  ];
  const surfaces = ["color-page", "color-surface", "color-surface-raised", "color-surface-subtle"];

  for (const mapName of ["theme-light", "theme-dark"]) {
    const theme = readHexThemeMap(tokens, mapName);
    const themeChecks = [...checks];
    for (const foregroundName of ["color-text", "color-muted", "color-subtle"]) {
      for (const backgroundName of surfaces) {
        themeChecks.push([foregroundName, backgroundName, 4.5]);
      }
    }
    for (const backgroundName of surfaces) {
      themeChecks.push(["color-border-strong", backgroundName, 3]);
    }
    for (const [foregroundName, backgroundName, minimum] of themeChecks) {
      const foreground = theme.get(foregroundName);
      const background = theme.get(backgroundName);
      assert.ok(foreground, `${mapName} misses ${foregroundName}`);
      assert.ok(background, `${mapName} misses ${backgroundName}`);
      assert.ok(
        contrastRatio(foreground, background) >= minimum,
        `${mapName} ${foregroundName}/${backgroundName} must be at least ${minimum}:1`,
      );
    }
  }
});

test("temporary system chrome overrides restore the latest theme baseline", async () => {
  const calls = [];
  globalThis.window = {
    WineStockSystemChrome: {
      setDarkContent(enabled) {
        calls.push(enabled);
      },
    },
  };
  const chromeSourceUrl = new URL("../src/shell/systemChrome.ts", import.meta.url);
  const chromeSource = await readFile(chromeSourceUrl, "utf8");
  const chromeTranspiled = ts.transpileModule(chromeSource, {
    fileName: chromeSourceUrl.pathname,
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
    },
  });
  const chromeModuleUrl = `data:text/javascript;base64,${Buffer.from(chromeTranspiled.outputText).toString("base64")}`;
  const { acquireSystemChromeDarkContent, setSystemChromeBaseDarkContent } = await import(
    chromeModuleUrl
  );

  setSystemChromeBaseDarkContent(false);
  const releaseFirst = acquireSystemChromeDarkContent();
  const releaseSecond = acquireSystemChromeDarkContent();
  setSystemChromeBaseDarkContent(true);
  releaseFirst();
  setSystemChromeBaseDarkContent(false);
  releaseSecond();
  releaseSecond();

  assert.deepEqual(calls, [false, true, true, true, true, true, false]);
  delete globalThis.window;
});
