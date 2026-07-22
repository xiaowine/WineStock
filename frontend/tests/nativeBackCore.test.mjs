import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const coreSourceUrl = new URL("../src/navigation/nativeBackCore.ts", import.meta.url);
const coreSource = await readFile(coreSourceUrl, "utf8");
const transpiledCore = ts.transpileModule(coreSource, {
  fileName: coreSourceUrl.pathname,
  compilerOptions: {
    module: ts.ModuleKind.ES2022,
    target: ts.ScriptTarget.ES2022,
    verbatimModuleSyntax: true,
  },
});
const coreModuleUrl = `data:text/javascript;base64,${Buffer.from(transpiledCore.outputText).toString("base64")}`;
const { createNativeBackRegistry, installNativeBackCoordinator } = await import(coreModuleUrl);

const request = (requestId = "page-1:1", canGoBack = true) => ({ requestId, canGoBack });
const handled = (reason) => ({ handled: true, reason });

test("registry dispatches higher priorities before lower handlers", async () => {
  const calls = [];
  const registry = createNativeBackRegistry();
  registry.register({
    id: "drawer",
    priority: 300,
    isActive: () => true,
    handle: () => {
      calls.push("drawer");
      return handled("drawer");
    },
  });
  registry.register({
    id: "dialog",
    priority: 400,
    isActive: () => true,
    handle: () => {
      calls.push("dialog");
      return handled("dialog");
    },
  });

  assert.deepEqual(await registry.dispatch(request()), handled("dialog"));
  assert.deepEqual(calls, ["dialog"]);
});

test("registry uses LIFO within a priority and unregister is idempotent", async () => {
  const calls = [];
  const registry = createNativeBackRegistry();
  registry.register({
    id: "first-dialog",
    priority: 400,
    isActive: () => true,
    handle: () => {
      calls.push("first");
      return handled("dialog");
    },
  });
  const unregisterSecond = registry.register({
    id: "second-dialog",
    priority: 400,
    isActive: () => true,
    handle: () => {
      calls.push("second");
      return handled("dialog");
    },
  });

  await registry.dispatch(request("page-1:1"));
  unregisterSecond();
  unregisterSecond();
  await registry.dispatch(request("page-1:2"));

  assert.deepEqual(calls, ["second", "first"]);
});

test("registry skips inactive handlers and continues after handled=false", async () => {
  const calls = [];
  const registry = createNativeBackRegistry();
  registry.register({
    id: "fallback",
    priority: 100,
    isActive: () => true,
    handle: () => {
      calls.push("fallback");
      return handled("route-history");
    },
  });
  registry.register({
    id: "continue",
    priority: 300,
    isActive: () => true,
    handle: () => {
      calls.push("continue");
      return { handled: false };
    },
  });
  registry.register({
    id: "inactive",
    priority: 500,
    isActive: () => false,
    handle: () => {
      throw new Error("inactive handler must not run");
    },
  });

  assert.deepEqual(await registry.dispatch(request()), handled("route-history"));
  assert.deepEqual(calls, ["continue", "fallback"]);
});

test("busy handler consumes back without reaching lower layers", async () => {
  let lowerLayerCalled = false;
  const registry = createNativeBackRegistry();
  registry.register({
    id: "lower-layer",
    priority: 100,
    isActive: () => true,
    handle: () => {
      lowerLayerCalled = true;
      return handled("route-history");
    },
  });
  registry.register({
    id: "busy-dialog",
    priority: 400,
    isActive: () => true,
    handle: () => handled("busy-dialog"),
  });

  assert.deepEqual(await registry.dispatch(request()), handled("busy-dialog"));
  assert.equal(lowerLayerCalled, false);
});

test("handler exceptions are reported and safely consumed", async () => {
  const reported = [];
  const registry = createNativeBackRegistry({
    onHandlerError: (registration, error) => reported.push([registration.id, error.message]),
  });
  registry.register({
    id: "broken-handler",
    priority: 500,
    isActive: () => true,
    handle: () => {
      throw new Error("broken");
    },
  });

  assert.deepEqual(await registry.dispatch(request()), handled("handler-error"));
  assert.deepEqual(reported, [["broken-handler", "broken"]]);
});

test("coordinator routes only when canGoBack and resolves each request once", async () => {
  const registry = createNativeBackRegistry();
  const resolutions = [];
  let listener;
  let navigationCount = 0;
  const dispose = await installNativeBackCoordinator({
    registry,
    subscribe: async (nextListener) => {
      listener = nextListener;
      return () => undefined;
    },
    resolve: async (resolution) => resolutions.push(resolution),
    navigateBack: () => {
      navigationCount += 1;
    },
  });

  listener(request("page-1:1", true));
  listener(request("page-1:1", true));
  listener(request("page-1:2", false));
  await flushAsyncWork();

  assert.equal(navigationCount, 1);
  assert.deepEqual(resolutions, [
    { requestId: "page-1:1", handled: true, reason: "route-history" },
    { requestId: "page-1:2", handled: false, reason: "unhandled" },
  ]);
  dispose();
});

test("dispose stops events and suppresses an in-flight async resolution", async () => {
  const registry = createNativeBackRegistry();
  const resolutions = [];
  let listener;
  let releaseHandler;
  let stopCount = 0;
  registry.register({
    id: "async-dialog",
    priority: 400,
    isActive: () => true,
    handle: () => new Promise((resolve) => (releaseHandler = resolve)),
  });
  const dispose = await installNativeBackCoordinator({
    registry,
    subscribe: async (nextListener) => {
      listener = nextListener;
      return () => {
        stopCount += 1;
      };
    },
    resolve: async (resolution) => resolutions.push(resolution),
    navigateBack: () => undefined,
  });

  listener(request());
  dispose();
  dispose();
  releaseHandler(handled("dialog"));
  listener(request("page-1:2"));
  await flushAsyncWork();

  assert.equal(stopCount, 1);
  assert.deepEqual(resolutions, []);
  assert.deepEqual(await registry.dispatch(request("page-1:3")), { handled: false });
});

test("subscription failure removes the route handler", async () => {
  const registry = createNativeBackRegistry();
  let navigationCount = 0;

  await assert.rejects(
    installNativeBackCoordinator({
      registry,
      subscribe: async () => {
        throw new Error("subscription failed");
      },
      resolve: async () => undefined,
      navigateBack: () => {
        navigationCount += 1;
      },
    }),
    /subscription failed/,
  );

  assert.deepEqual(await registry.dispatch(request()), { handled: false });
  assert.equal(navigationCount, 0);
});

async function flushAsyncWork() {
  await new Promise((resolve) => setImmediate(resolve));
  await new Promise((resolve) => setImmediate(resolve));
}
