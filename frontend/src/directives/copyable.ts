// 本文件拥有 v-copyable 自定义指令：点击宿主元素即复制并发全局 Notice。
// 项目规范：一切"点击复制"交互必须经本指令或 clipboard/copyText 工具，禁止手写剪贴板代码
//（规范见 frontend/docs/ui-consistency-checklist.md）。
import type { Directive } from "vue";
import { copyWithFeedback } from "../clipboard/copyText";
import { normalizeCopyableValue, type CopyRequest } from "../clipboard/model";

interface CopyableHost extends HTMLElement {
  __copyableRequest?: CopyRequest | null;
  __copyableCleanup?: () => void;
}

/** 原生可交互元素不需要指令补 role/tabindex/键盘触发。 */
function isNativelyInteractive(el: HTMLElement): boolean {
  return ["BUTTON", "A", "INPUT", "TEXTAREA", "SUMMARY"].includes(el.tagName);
}

function resolveRequest(el: CopyableHost, value: unknown): CopyRequest | null {
  return normalizeCopyableValue(value, el.textContent ?? "");
}

/** 用户正在宿主内手动选择文本时不触发复制，避免抢走拖选操作。 */
function hasActiveSelectionInside(el: HTMLElement): boolean {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.anchorNode) return false;
  return el.contains(selection.anchorNode);
}

/**
 * 用法：`v-copyable`（复制元素文本）、`v-copyable="text"` 或
 * `v-copyable="{ text, label }"`（label 进入 Notice 文案，如"连接地址已复制"）。
 */
export const copyableDirective: Directive<HTMLElement, unknown> = {
  mounted(el: CopyableHost, binding) {
    el.__copyableRequest = resolveRequest(el, binding.value);

    const onClick = () => {
      if (hasActiveSelectionInside(el)) return;
      const request = el.__copyableRequest;
      if (request) void copyWithFeedback(request);
    };
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      onClick();
    };

    el.addEventListener("click", onClick);
    if (!isNativelyInteractive(el)) {
      // 文本宿主自动获得可发现性外观（styles/shared/_copyable.scss）；按钮宿主保持自身样式。
      el.classList.add("copyable");
      el.setAttribute("role", "button");
      if (!el.hasAttribute("tabindex")) el.setAttribute("tabindex", "0");
      el.addEventListener("keydown", onKeydown);
    }
    el.__copyableCleanup = () => {
      el.removeEventListener("click", onClick);
      el.removeEventListener("keydown", onKeydown);
    };
  },
  updated(el: CopyableHost, binding) {
    el.__copyableRequest = resolveRequest(el, binding.value);
  },
  unmounted(el: CopyableHost) {
    el.__copyableCleanup?.();
    delete el.__copyableRequest;
    delete el.__copyableCleanup;
  },
};
