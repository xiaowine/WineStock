// 本模块拥有全局应用更新 Dialog 的短生命周期状态；更新检查和安装仍由当前平台 Shell 执行。
import { readonly, ref } from "vue";
import type { AppUpdateCheckResult } from "../shell/contract";

export type AppUpdateDialogSource = "startup" | "preferences";

const mutableOpen = ref(false);
const mutableResult = ref<AppUpdateCheckResult | null>(null);
const mutableNested = ref(false);

export const appUpdateDialogOpen = readonly(mutableOpen);
export const appUpdateDialogResult = readonly(mutableResult);
export const appUpdateDialogNested = readonly(mutableNested);

/** 打开更新 Dialog；没有新版本时不改变当前浮层状态。 */
export function openAppUpdateDialog(
  result: AppUpdateCheckResult,
  source: AppUpdateDialogSource,
): void {
  if (!result.latestVersion) return;
  mutableResult.value = result;
  mutableNested.value = source === "preferences";
  mutableOpen.value = true;
}

/** 请求关闭更新 Dialog；结果在离场动画完成后清理。 */
export function closeAppUpdateDialog(): void {
  mutableOpen.value = false;
}

/** 清理已经完成离场的更新 Dialog。 */
export function clearAppUpdateDialog(): void {
  if (mutableOpen.value) return;
  mutableResult.value = null;
  mutableNested.value = false;
}
