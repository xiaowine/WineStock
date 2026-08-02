// 本文件选择当前平台的 Shell Bridge 传输适配；它不保存运行状态或调用业务 HTTP API。
import { isTauri } from "@tauri-apps/api/core";
import type { ShellBridge } from "./contract";
import { createTauriShellBridge } from "./transports/tauri";
import { createWebShellBridge } from "./transports/web";

/** 优先使用平台在页面启动前注入的桥；Tauri/Web 传输按运行时宿主选择。 */
export function createShellBridge(): ShellBridge {
  if (window.__WINESTOCK_SHELL_BRIDGE__) {
    return window.__WINESTOCK_SHELL_BRIDGE__;
  }
  return isTauri() ? createTauriShellBridge() : createWebShellBridge();
}
