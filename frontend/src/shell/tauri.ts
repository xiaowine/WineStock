// 本文件实现 Desktop Tauri v2 的 Shell Bridge 传输；它不管理运行状态或代理业务 HTTP 请求。
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ApplyRuntimeConfigResult,
  EditableRuntimeConfig,
  RuntimeConfigValidationResult,
  RuntimeSnapshot,
  ShellBridge,
} from "./contract";

const RUNTIME_STATE_EVENT = "winestock-runtime-state-changed";
const APP_RESUMED_EVENT = "winestock-app-resumed";

/** 把统一 Shell Bridge v1 映射到受 capability 限制的 Tauri command 与 event。 */
export function createTauriShellBridge(): ShellBridge {
  return {
    getRuntimeSnapshot() {
      return invoke<RuntimeSnapshot>("shell_get_runtime_snapshot");
    },
    validateRuntimeConfig(config) {
      return invoke<RuntimeConfigValidationResult>("shell_validate_runtime_config", { config });
    },
    applyRuntimeConfig(config) {
      return invoke<ApplyRuntimeConfigResult>("shell_apply_runtime_config", { config });
    },
    startLocalService() {
      return invoke<RuntimeSnapshot>("shell_start_local_service");
    },
    stopLocalService() {
      return invoke<RuntimeSnapshot>("shell_stop_local_service");
    },
    restartLocalService() {
      return invoke<RuntimeSnapshot>("shell_restart_local_service");
    },
    frontendReady() {
      return invoke<void>("shell_frontend_ready");
    },
    openExternal(url) {
      return invoke<void>("shell_open_external", { url });
    },
    async onRuntimeStateChanged(listener) {
      return listen<RuntimeSnapshot>(RUNTIME_STATE_EVENT, (event) => listener(event.payload));
    },
    async onAppResumed(listener) {
      return listen(APP_RESUMED_EVENT, () => listener());
    },
  };
}

// 让 TypeScript 保持对 command 参数形状的约束；实际序列化仍由 Tauri 完成。
void (null as EditableRuntimeConfig | null);
