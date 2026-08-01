// 本文件选择当前平台的 Shell Bridge 传输适配；它不保存运行状态或调用业务 HTTP API。
import type { ShellBridge } from "./contract";
import { createTauriShellBridge } from "./transports/tauri";
import { createWebShellBridge } from "./transports/web";

/** 优先使用平台在页面启动前注入的版本化桥，否则使用普通 Web fallback。 */
export function createShellBridge(): ShellBridge {
  if (window.__WINESTOCK_SHELL_BRIDGE__) {
    return window.__WINESTOCK_SHELL_BRIDGE__;
  }
  return import.meta.env.MODE === "desktop" ? createTauriShellBridge() : createWebShellBridge();
}
