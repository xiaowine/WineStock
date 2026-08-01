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

const { assertCompatibleRuntimeSnapshot, ShellBridgeContractError } = await loadModule(
  "../src/shell/contract.ts",
);
const { normalizeShellBridgeTransportError, ShellBridgeTransportError } = await loadModule(
  "../src/shell/bridgeError.ts",
);

function snapshot(overrides = {}) {
  const { config: configOverride, service: serviceOverride, capabilities: capabilityOverride, ...rest } =
    overrides;
  return {
    protocolVersion: 1,
    platform: "desktop",
    configStatus: "configured",
    config: {
      mode: "self-hosted",
      bindHost: "127.0.0.1",
      port: 17890,
      remoteBaseUrl: "",
      ...(configOverride ?? {}),
    },
    initialized: true,
    service: {
      ownership: "local",
      phase: "running",
      apiBaseUrl: "http://127.0.0.1:17890",
      ...(serviceOverride ?? {}),
    },
    capabilities: {
      startLocalService: true,
      stopLocalService: true,
      restartLocalService: true,
      nativeBack: false,
      openExternal: true,
      serverMode: false,
      ...(capabilityOverride ?? {}),
    },
    ...rest,
  };
}

test("accepts a valid local running snapshot", () => {
  assert.doesNotThrow(() => assertCompatibleRuntimeSnapshot(snapshot()));
});

test("accepts the Web and Android capability projections", () => {
  assert.doesNotThrow(() =>
    assertCompatibleRuntimeSnapshot(
      snapshot({
        platform: "web",
        configStatus: "unconfigured",
        initialized: false,
        service: { phase: "stopped", apiBaseUrl: undefined },
        capabilities: {
          startLocalService: false,
          stopLocalService: false,
          restartLocalService: false,
          nativeBack: false,
          openExternal: true,
          serverMode: false,
        },
      }),
    ),
  );
  assert.doesNotThrow(() =>
    assertCompatibleRuntimeSnapshot(
      snapshot({
        platform: "android",
        config: { mode: "client-only", remoteBaseUrl: "https://server.example.test:17890" },
        service: {
          ownership: "remote",
          phase: "running",
          apiBaseUrl: "https://server.example.test:17890",
        },
        capabilities: {
          startLocalService: false,
          stopLocalService: false,
          restartLocalService: false,
          nativeBack: true,
          openExternal: true,
          serverMode: false,
        },
      }),
    ),
  );
});

test("rejects unsafe API addresses and invalid state combinations", () => {
  for (const candidate of [
    snapshot({ service: { apiBaseUrl: "http://0.0.0.0:17890" } }),
    snapshot({ service: { apiBaseUrl: "http://127.0.0.1:0" } }),
    snapshot({ initialized: true, configStatus: "invalid" }),
    snapshot({ service: { phase: "running", apiBaseUrl: undefined } }),
    snapshot({ service: { ownership: "remote" }, capabilities: { startLocalService: true } }),
  ]) {
    assert.throws(
      () => assertCompatibleRuntimeSnapshot(candidate),
      (error) => error instanceof ShellBridgeContractError && error.code === "invalid_bridge_payload",
    );
  }
});

test("normalizes Tauri JSON string and object errors to stable codes", () => {
  const fromString = normalizeShellBridgeTransportError(
    JSON.stringify({ code: "port_in_use", message: "端口已被占用" }),
  );
  assert.equal(fromString.code, "port_in_use");
  assert.equal(fromString.message, "端口已被占用");

  const fromObject = normalizeShellBridgeTransportError({
    code: "service_start_failed",
    message: "服务启动失败",
  });
  assert.ok(fromObject instanceof ShellBridgeTransportError);
  assert.equal(fromObject.code, "service_start_failed");
});

test("normalizes malformed transport errors without leaking an unstructured rejection", () => {
  const normalized = normalizeShellBridgeTransportError(new Error("invoke failed"));
  assert.equal(normalized.code, "invalid_bridge_payload");
  assert.equal(normalized.message, "invoke failed");
});
