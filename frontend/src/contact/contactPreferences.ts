// 本文件拥有联系与反馈入口的前端显示偏好；它只使用 localStorage，不进入 Shell 偏好契约。
import { readonly, ref } from "vue";

const STORAGE_KEY = "winestock.contact-entry.visible.v1";
const visibleState = ref(readStoredVisibility());

export const contactEntryVisible = readonly(visibleState);

export function setContactEntryVisible(visible: boolean): void {
  visibleState.value = visible;
  try {
    window.localStorage.setItem(STORAGE_KEY, visible ? "1" : "0");
  } catch {
    // 存储不可用时保留本次会话内的显示状态。
  }
}

function readStoredVisibility(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) !== "0";
  } catch {
    return true;
  }
}
