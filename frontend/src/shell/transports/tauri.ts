// 本文件实现 Desktop Tauri v2 的 Shell Bridge 前端传输适配；它不管理运行状态或代理业务 HTTP 请求。
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { normalizeShellBridgeTransportError } from "./bridgeError";
import { assertDesktopPreferences } from "../contract";
import type {
  ApplyRuntimeConfigResult,
  DesktopPreferences,
  EditableRuntimeConfig,
  RuntimeConfigValidationResult,
  RuntimeSnapshot,
  ShellBridge,
} from "../contract";

const RUNTIME_STATE_EVENT = "winestock-runtime-state-changed";
const APP_RESUMED_EVENT = "winestock-app-resumed";

/** 把统一 Shell Bridge v1 映射到受 capability 限制的 Tauri command 与 event。 */
export function createTauriShellBridge(): ShellBridge {
  return {
    getRuntimeSnapshot() {
      return invokeShell<RuntimeSnapshot>("shell_get_runtime_snapshot");
    },
    validateRuntimeConfig(config) {
      return invokeShell<RuntimeConfigValidationResult>("shell_validate_runtime_config", {
        config,
      });
    },
    applyRuntimeConfig(config) {
      return invokeShell<ApplyRuntimeConfigResult>("shell_apply_runtime_config", { config });
    },
    startLocalService() {
      return invokeShell<RuntimeSnapshot>("shell_start_local_service");
    },
    stopLocalService() {
      return invokeShell<RuntimeSnapshot>("shell_stop_local_service");
    },
    restartLocalService() {
      return invokeShell<RuntimeSnapshot>("shell_restart_local_service");
    },
    repairFirewall() {
      return invokeShell<RuntimeSnapshot>("shell_repair_firewall");
    },
    frontendReady() {
      return invokeShell<void>("shell_frontend_ready");
    },
    openExternal(url) {
      return invokeShell<void>("shell_open_external", { url });
    },
    async getDesktopPreferences() {
      const preferences = await invokeShell<DesktopPreferences>("shell_get_desktop_preferences");
      assertDesktopPreferences(preferences);
      return preferences;
    },
    async setDesktopPreferences(preferences) {
      const next = await invokeShell<DesktopPreferences>("shell_set_desktop_preferences", {
        preferences,
      });
      assertDesktopPreferences(next);
      return next;
    },
    async onRuntimeStateChanged(listener) {
      return listen<RuntimeSnapshot>(RUNTIME_STATE_EVENT, (event) => listener(event.payload));
    },
    async onAppResumed(listener) {
      return listen(APP_RESUMED_EVENT, () => listener());
    },
  };
}

function invokeShell<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    throw normalizeShellBridgeTransportError(error);
  });
}

// 让 TypeScript 保持对 command 参数形状的约束；实际序列化仍由 Tauri 完成。
void (null as EditableRuntimeConfig | null);
