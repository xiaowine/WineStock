// 本模块拥有应用更新的前端本机偏好；它只控制启动时是否检查，不改变手动检查和安装流程。
import { readonly, ref } from "vue";

const STORAGE_KEY = "winestock.updates.auto-check.v1";
const autoCheckState = ref(readStoredAutoCheck());

export const autoUpdateCheckEnabled = readonly(autoCheckState);

/** 保存启动时自动检查更新的偏好；存储不可用时保留当前会话状态。 */
export function setAutoUpdateCheckEnabled(enabled: boolean): void {
  autoCheckState.value = enabled;
  try {
    window.localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
  } catch {
    // 本机会话内的开关仍然立即生效。
  }
}

function readStoredAutoCheck(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) !== "0";
  } catch {
    return true;
  }
}
