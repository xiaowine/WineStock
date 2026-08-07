// 本文件拥有 v-overlay-scrollbar 自定义指令：显式登记需要移动端浮层滚动条的宿主。
import type { Directive } from "vue";
import {
  registerOverlayScrollbar,
  unregisterOverlayScrollbar,
} from "../bootstrap/overlayScrollbars";

/** 滚动容器由组件声明所有权，指令只负责注册生命周期。 */
export const overlayScrollbarDirective: Directive<HTMLElement> = {
  mounted(element) {
    registerOverlayScrollbar(element);
  },
  unmounted(element) {
    unregisterOverlayScrollbar(element);
  },
};
