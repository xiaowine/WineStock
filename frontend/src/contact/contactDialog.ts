// 本文件拥有联系 Dialog 的共享开关，允许顶栏、账户菜单和业务页面复用同一层浮层。
import { readonly, ref } from "vue";

const openState = ref(false);

export const contactDialogOpen = readonly(openState);

export function openContactDialog(): void {
  openState.value = true;
}

export function closeContactDialog(): void {
  openState.value = false;
}
