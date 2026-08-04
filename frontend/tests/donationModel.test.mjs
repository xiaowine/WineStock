import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

async function loadModel() {
  const sourceUrl = new URL("../src/donation/model.ts", import.meta.url);
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
  return import(moduleUrl);
}

const model = await loadModel();
const baseTime = new Date("2026-01-01T00:00:00.000Z");

function addOpens(state, count) {
  let next = state;
  for (let index = 0; index < count; index += 1) {
    next = model.applyAppOpen(next, new Date(baseTime.getTime() + index));
  }
  return next;
}

test("app open milestones trigger at 10 and 50 cumulative sessions", () => {
  let state = addOpens(model.createDefaultDonationPromptState(), 9);
  assert.equal(model.findDonationPrompt(state, baseTime, true), null);

  state = model.applyAppOpen(state, baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "app-open-10",
    reason: "app-open",
  });

  state = model.markDonationPromptShown(state, "app-open-10", baseTime);
  state = addOpens(state, 40);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "app-open-50",
    reason: "app-open",
  });
});

test("item milestones trigger at 50, 100 and 300 created items", () => {
  let state = model.createDefaultDonationPromptState();
  state = model.applyItemsCreated(state, 49, baseTime);
  assert.equal(model.findDonationPrompt(state, baseTime, true), null);

  state = model.applyItemsCreated(state, 1, baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-50",
    reason: "items-created",
  });

  state = model.markDonationPromptShown(state, "items-created-50", baseTime);
  state = model.applyItemsCreated(state, 250, new Date(baseTime.getTime() + 1));
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-100",
    reason: "items-created",
  });
  state = model.markDonationPromptShown(state, "items-created-100", baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-300",
    reason: "items-created",
  });
});

test("reached milestones queue without repeating the same prompt", () => {
  let state = model.applyItemsCreated(model.createDefaultDonationPromptState(), 300, baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-50",
    reason: "items-created",
  });
  state = model.markDonationPromptShown(state, "items-created-50", baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-100",
    reason: "items-created",
  });
  state = model.markDonationPromptShown(state, "items-created-100", baseTime);
  assert.deepEqual(model.findDonationPrompt(state, baseTime, true), {
    milestone: "items-created-300",
    reason: "items-created",
  });
});

test("cooldown blocks prompts without losing reached milestones", () => {
  let state = model.applyItemsCreated(model.createDefaultDonationPromptState(), 50, baseTime);
  state = model.markDonationPromptShown(state, "items-created-50", baseTime);
  state = model.snoozeDonationPrompt(state, 30, baseTime);
  assert.equal(model.findDonationPrompt(state, new Date("2026-01-15T00:00:00.000Z"), true), null);
  assert.deepEqual(
    model.findDonationPrompt(state, new Date("2026-02-01T00:00:00.000Z"), true),
    null,
  );
});

test("invalid counts and damaged storage fall back safely", () => {
  const state = model.createDefaultDonationPromptState();
  assert.equal(model.applyItemsCreated(state, 0, baseTime).totalItemsCreated, 0);
  assert.equal(model.applyItemsCreated(state, -1, baseTime).totalItemsCreated, 0);
  assert.equal(model.applyItemsCreated(state, 1.5, baseTime).totalItemsCreated, 0);

  const normalized = model.normalizeDonationPromptState({
    version: 1,
    totalAppOpens: -10,
    totalItemsCreated: "300",
    acknowledgedMilestones: ["items-created-50", "unknown"],
    reachedMilestonesAt: { "items-created-50": "not-a-date" },
    autoPromptDisabled: true,
  });
  assert.equal(normalized.totalAppOpens, 0);
  assert.equal(normalized.totalItemsCreated, 0);
  assert.deepEqual(normalized.acknowledgedMilestones, ["items-created-50"]);
  assert.equal(normalized.autoPromptDisabled, true);
});

test("disabled donation configuration never produces an automatic prompt", () => {
  let state = model.applyItemsCreated(model.createDefaultDonationPromptState(), 300, baseTime);
  assert.equal(model.findDonationPrompt(state, baseTime, false), null);
});
