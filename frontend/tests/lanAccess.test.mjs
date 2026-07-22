import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import ts from "typescript";

const sourceUrl = new URL("../src/shell/lanAccess.ts", import.meta.url);
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
const { getUsableLanAccessUrls, normalizeLanAccessUrls } = await import(moduleUrl);

function createSnapshot(overrides = {}) {
  const snapshot = {
    protocolVersion: 1,
    platform: "desktop",
    configStatus: "configured",
    config: {
      mode: "server-mode",
      bindHost: "0.0.0.0",
      port: 17890,
      remoteBaseUrl: "",
    },
    createdDefault: false,
    service: {
      ownership: "local",
      phase: "running",
      apiBaseUrl: "http://127.0.0.1:17890",
      boundAddress: "0.0.0.0:17890",
      lanAccessUrls: ["http://192.168.1.23:17890"],
    },
    capabilities: {
      startLocalService: true,
      stopLocalService: true,
      restartLocalService: true,
      nativeBack: false,
      openExternal: true,
      serverMode: true,
    },
  };

  return {
    ...snapshot,
    ...overrides,
    config: { ...snapshot.config, ...overrides.config },
    service: { ...snapshot.service, ...overrides.service },
    capabilities: { ...snapshot.capabilities, ...overrides.capabilities },
  };
}

test("normalizes valid LAN origins, removes duplicates, and preserves Shell order", () => {
  assert.deepEqual(
    normalizeLanAccessUrls([
      " HTTP://192.168.1.23:17890/ ",
      "http://192.168.1.23:17890",
      "http://[fd00::23]:17890/",
      "https://wine-host.local:443/",
    ]),
    ["http://192.168.1.23:17890", "http://[fd00::23]:17890", "https://wine-host.local"],
  );
});

test("rejects placeholders, wildcard, loopback, credentials, paths, and malformed values", () => {
  assert.deepEqual(
    normalizeLanAccessUrls([
      "http://<局域网地址>:17890",
      "http://局域网地址:17890",
      "http://0.0.0.0:17890",
      "http://[::]:17890",
      "http://localhost:17890",
      "http://api.localhost:17890",
      "http://127.8.9.10:17890",
      "http://[::1]:17890",
      "http://user:secret@192.168.1.23:17890",
      "http://192.168.1.23:17890/api",
      "http://192.168.1.23:17890/?token=secret",
      "ftp://192.168.1.23:17890",
      "not a url",
    ]),
    [],
  );
});

test("only exposes addresses for an active local server-mode service with capability", () => {
  assert.deepEqual(getUsableLanAccessUrls(createSnapshot()), ["http://192.168.1.23:17890"]);
  assert.deepEqual(getUsableLanAccessUrls(createSnapshot({ config: { mode: "self-hosted" } })), []);
  assert.deepEqual(
    getUsableLanAccessUrls(createSnapshot({ service: { ownership: "remote" } })),
    [],
  );
  assert.deepEqual(getUsableLanAccessUrls(createSnapshot({ service: { phase: "starting" } })), []);
  assert.deepEqual(
    getUsableLanAccessUrls(createSnapshot({ capabilities: { serverMode: false } })),
    [],
  );
});

test("returns no address when the Shell only publishes the former Web placeholder", () => {
  assert.deepEqual(
    getUsableLanAccessUrls(
      createSnapshot({ service: { lanAccessUrls: ["http://<局域网地址>:17890"] } }),
    ),
    [],
  );
});
