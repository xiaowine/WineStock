// 本文件选择当前平台的 Shell Bridge 传输；它不保存运行状态或调用业务 HTTP API。
import type { ShellBridge } from "./contract";
import { createWebShellBridge } from "./web";

/** 优先使用平台在页面启动前注入的版本化桥，否则使用普通 Web fallback。 */
export function createShellBridge(): ShellBridge {
  return window.__WINESTOCK_SHELL_BRIDGE__ ?? createWebShellBridge();
}
