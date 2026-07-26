import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

async function loadModule(relativePath) {
  const sourceUrl = new URL(relativePath, import.meta.url);
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

const { isRuntimeServiceReady, isRuntimeSetupFinished, shouldEnterSetupWizard } = await loadModule(
  "../src/shell/runtimeReadiness.ts",
);
const { bridgeReturnToToAuthRedirect, isSafeInternalPath, resolveRuntimeSettingsLeave } =
  await loadModule("../src/pages/runtime-settings/leave.ts");

function snapshot(overrides = {}) {
  const { service: serviceOverride, ...rest } = overrides;
  return {
    configStatus: "configured",
    initialized: true,
    service: {
      apiBaseUrl: "http://127.0.0.1:17890",
      ...(serviceOverride ?? {}),
    },
    ...rest,
  };
}

test("unready when snapshot is null", () => {
  assert.equal(isRuntimeServiceReady(null), false);
  assert.equal(isRuntimeSetupFinished(null), false);
});

test("unready when configStatus is unconfigured or invalid", () => {
  assert.equal(
    isRuntimeServiceReady(
      snapshot({ configStatus: "unconfigured", service: { apiBaseUrl: undefined } }),
    ),
    false,
  );
  assert.equal(
    isRuntimeSetupFinished(
      snapshot({ configStatus: "unconfigured", service: { apiBaseUrl: undefined } }),
    ),
    false,
  );
});

test("service readiness is independent from Shell initialization", () => {
  assert.equal(isRuntimeServiceReady(snapshot({ initialized: false })), true);
  assert.equal(isRuntimeServiceReady(snapshot({ initialized: true })), true);
});

test("setup finished follows Shell initialized flag", () => {
  assert.equal(isRuntimeSetupFinished(snapshot({ initialized: false })), false);
  assert.equal(isRuntimeSetupFinished(snapshot({ initialized: true })), true);
});

test("uninitialized Shell snapshot stays in setup even if it exposes an API", () => {
  const uninitialized = snapshot({ initialized: false });
  assert.equal(isRuntimeServiceReady(uninitialized), true);
  assert.equal(isRuntimeSetupFinished(uninitialized), false);
  const afterApply = snapshot({ initialized: true });
  assert.equal(isRuntimeSetupFinished(afterApply), true);
});

test("setup wizard only serves the unconfigured uninitialized state", () => {
  assert.equal(
    shouldEnterSetupWizard(
      snapshot({ configStatus: "unconfigured", initialized: false, service: {} }),
    ),
    true,
  );
  // invalid（配置损坏）走运行设置修复路径，不进向导。
  assert.equal(
    shouldEnterSetupWizard(snapshot({ configStatus: "invalid", initialized: false, service: {} })),
    false,
  );
  // 已配置但服务未就绪同样维持运行设置页。
  assert.equal(shouldEnterSetupWizard(snapshot({ initialized: true })), false);
  // 快照缺失（Shell Bridge 初始化失败）由运行设置页呈现错误。
  assert.equal(shouldEnterSetupWizard(null), false);
  assert.equal(shouldEnterSetupWizard(undefined), false);
});

test("safe internal path rejects external-looking values", () => {
  assert.equal(isSafeInternalPath("/dashboard"), true);
  assert.equal(isSafeInternalPath("//evil.example"), false);
});

test("bridge: business and auth returnTo", () => {
  assert.equal(bridgeReturnToToAuthRedirect("/dashboard"), "/dashboard");
  assert.equal(bridgeReturnToToAuthRedirect("/auth?redirect=/items"), "/items");
  assert.equal(bridgeReturnToToAuthRedirect("/settings/runtime"), undefined);
});

test("leave: unfinished stays; finished anonymous goes auth", () => {
  assert.deepEqual(
    resolveRuntimeSettingsLeave({
      returnTo: "/dashboard",
      setupFinished: false,
      authenticated: false,
      returnToRouteValid: true,
    }),
    { kind: "stay" },
  );
  assert.deepEqual(
    resolveRuntimeSettingsLeave({
      returnTo: "/dashboard",
      setupFinished: true,
      authenticated: false,
      returnToRouteValid: true,
    }),
    { kind: "auth", redirect: "/dashboard" },
  );
});

test("leave: authenticated uses path or default app", () => {
  assert.deepEqual(
    resolveRuntimeSettingsLeave({
      returnTo: "/dashboard",
      setupFinished: true,
      authenticated: true,
      returnToRouteValid: true,
    }),
    { kind: "path", path: "/dashboard" },
  );
  assert.deepEqual(
    resolveRuntimeSettingsLeave({
      returnTo: undefined,
      setupFinished: false,
      authenticated: true,
      returnToRouteValid: false,
    }),
    { kind: "default-app" },
  );
});
