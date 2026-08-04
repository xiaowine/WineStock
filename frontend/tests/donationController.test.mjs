import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

globalThis.window = {
  localStorage: { getItem: () => null, setItem: () => undefined },
  sessionStorage: { getItem: () => null, setItem: () => undefined },
  addEventListener: () => undefined,
};

async function createTsModuleUrl(relativePath, replacements = []) {
  const sourceUrl = new URL(relativePath, import.meta.url);
  let source = await readFile(sourceUrl, "utf8");
  for (const [from, to] of replacements) source = source.replaceAll(from, to);
  const transpiled = ts.transpileModule(source, {
    fileName: sourceUrl.pathname,
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
      verbatimModuleSyntax: true,
    },
  });
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`;
  return moduleUrl;
}

const modelUrl = await createTsModuleUrl("../src/donation/model.ts");
const model = await import(modelUrl);
const configUrl = await createTsModuleUrl("../src/donation/config.ts", [
  ["import.meta.env.VITE_DONATION_WECHAT_CONTENT", "undefined"],
  ["import.meta.env.VITE_DONATION_ALIPAY_CONTENT", "undefined"],
]);
const storageUrl = await createTsModuleUrl("../src/donation/storage.ts", [
  ['from "./model"', `from ${JSON.stringify(modelUrl)}`],
]);
const controllerUrl = await createTsModuleUrl("../src/donation/controller.ts", [
  ['from "./config"', `from ${JSON.stringify(configUrl)}`],
  ['from "./model"', `from ${JSON.stringify(modelUrl)}`],
  ['from "./storage"', `from ${JSON.stringify(storageUrl)}`],
]);
const controllerModule = await import(controllerUrl);
const testingUrl = await createTsModuleUrl("../src/donation/testing.ts");
const testing = await import(testingUrl);

function createMemoryStorage() {
  let state = model.createDefaultDonationPromptState();
  return {
    read: () => state,
    write: (next) => {
      state = next;
    },
    get state() {
      return state;
    },
  };
}

function resetSessionStorage() {
  globalThis.window = {
    localStorage: { getItem: () => null, setItem: () => undefined },
    sessionStorage: { getItem: () => null, setItem: () => undefined },
    addEventListener: () => undefined,
  };
}

test("item activity waits for the next app open before notifying", () => {
  resetSessionStorage();
  const memory = createMemoryStorage();
  const controller = new controllerModule.DonationController(true, memory);
  const decisions = [];
  controller.subscribe((decision) => decisions.push(decision));

  controller.recordItemsCreated(50);
  assert.equal(decisions.length, 0);
  assert.equal(memory.state.totalItemsCreated, 50);
  assert.equal(memory.state.reachedMilestonesAt["items-created-50"] !== undefined, true);

  controller.recordAppOpenOnce();
  assert.deepEqual(decisions, [{ milestone: "items-created-50", reason: "items-created" }]);
});

test("startup test parameters add activity without bypassing startup evaluation", () => {
  resetSessionStorage();
  const memory = createMemoryStorage();
  const controller = new controllerModule.DonationController(true, memory);
  const decisions = [];
  controller.subscribe((decision) => decisions.push(decision));

  controller.recordTestStartup({ additionalAppOpens: 9, itemsCreated: 50 });
  assert.equal(decisions.length, 0);
  controller.recordAppOpenOnce();
  assert.equal(decisions.length, 1);
  assert.equal(decisions[0].milestone, "app-open-10");
});

test("startup test parameters parse from the page URL", () => {
  assert.deepEqual(
    testing.readDonationStartupTestParams(
      "?donation_test_opens=9",
      "#/dashboard?donation_test_items=50",
    ),
    { additionalAppOpens: 9, itemsCreated: 50 },
  );
  assert.equal(testing.readDonationStartupTestParams("?donation_test_items=invalid"), null);
});
