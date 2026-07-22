// 本文件拥有应用壳账户弹层的开关与通用关闭时机；它不读取用户数据，也不执行账户操作。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { NativeBackPriority } from "../navigation/nativeBack";
import { useNativeBackHandler } from "./useNativeBackHandler";

/**
 * 管理账户弹层状态，并在路由变化或按下 Escape 时关闭。
 * Web Escape 与 Android 原生返回最终复用同一个 closeAccountMenu 状态转换。
 */
export function useAccountPopover() {
  const route = useRoute();
  const accountMenuOpen = ref(false);

  useNativeBackHandler({
    id: "account-popover",
    active: accountMenuOpen,
    priority: NativeBackPriority.Popover,
    handle: () => {
      if (!accountMenuOpen.value) return { handled: false };
      closeAccountMenu();
      return { handled: true, reason: "popover" };
    },
  });

  function toggleAccountMenu(): void {
    accountMenuOpen.value = !accountMenuOpen.value;
  }

  function closeAccountMenu(): void {
    accountMenuOpen.value = false;
  }

  function handleEscape(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      closeAccountMenu();
    }
  }

  watch(() => route.fullPath, closeAccountMenu);
  onMounted(() => document.addEventListener("keydown", handleEscape));
  onBeforeUnmount(() => document.removeEventListener("keydown", handleEscape));

  return {
    accountMenuOpen,
    closeAccountMenu,
    toggleAccountMenu,
  };
}
