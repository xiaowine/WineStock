import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/service/availabilityPolicy.ts", import.meta.url);
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
const {
  DEFAULT_SHELL_SIGNAL,
  LOCAL_AVAILABLE_CHECK_INTERVAL_MS,
  REMOTE_AVAILABLE_CHECK_INTERVAL_MS,
  UNAVAILABLE_CHECK_INTERVAL_MS,
  deriveStatusFromShellSignal,
  isShellDrivenStatus,
  pickNextCheckDelayMs,
  shouldConfirmBeforeUnavailable,
} = await import(moduleUrl);

const local = (phase) => ({ ownership: "local", phase });
const remote = { ownership: "remote" };

test("remote and local-running defer to HTTP probing", () => {
  assert.equal(deriveStatusFromShellSignal(remote), null);
  assert.equal(deriveStatusFromShellSignal(local("running")), null);
});

test("local lifecycle phases map to shell-driven statuses", () => {
  assert.equal(deriveStatusFromShellSignal(local("starting")), "checking");
  assert.equal(deriveStatusFromShellSignal(local("stopping")), "checking");
  assert.equal(deriveStatusFromShellSignal(local("failed")), "recovering");
  assert.equal(deriveStatusFromShellSignal(local("stopped")), "stopped");
});

test("default signal is remote so pure web keeps HTTP-only semantics", () => {
  assert.deepEqual(DEFAULT_SHELL_SIGNAL, { ownership: "remote" });
});

test("recovering and stopped suspend HTTP polling entirely", () => {
  assert.equal(isShellDrivenStatus("recovering"), true);
  assert.equal(isShellDrivenStatus("stopped"), true);
  assert.equal(pickNextCheckDelayMs(local("failed"), "recovering"), null);
  assert.equal(pickNextCheckDelayMs(local("stopped"), "stopped"), null);
});

test("available polling interval widens to watchdog only for local ownership", () => {
  assert.equal(
    pickNextCheckDelayMs(local("running"), "available"),
    LOCAL_AVAILABLE_CHECK_INTERVAL_MS,
  );
  assert.equal(pickNextCheckDelayMs(remote, "available"), REMOTE_AVAILABLE_CHECK_INTERVAL_MS);
  assert.ok(LOCAL_AVAILABLE_CHECK_INTERVAL_MS > REMOTE_AVAILABLE_CHECK_INTERVAL_MS);
});

test("non-available non-shell-driven statuses keep the recovery probing interval", () => {
  assert.equal(pickNextCheckDelayMs(remote, "unavailable"), UNAVAILABLE_CHECK_INTERVAL_MS);
  assert.equal(pickNextCheckDelayMs(remote, "checking"), UNAVAILABLE_CHECK_INTERVAL_MS);
  assert.equal(
    pickNextCheckDelayMs(local("running"), "unavailable"),
    UNAVAILABLE_CHECK_INTERVAL_MS,
  );
});

test("confirm-before-unavailable only applies once for local running available", () => {
  assert.equal(shouldConfirmBeforeUnavailable(local("running"), "available", false), true);
  assert.equal(shouldConfirmBeforeUnavailable(local("running"), "available", true), false);
  assert.equal(shouldConfirmBeforeUnavailable(remote, "available", false), false);
  assert.equal(shouldConfirmBeforeUnavailable(local("running"), "checking", false), false);
  assert.equal(shouldConfirmBeforeUnavailable(local("failed"), "available", false), false);
});
