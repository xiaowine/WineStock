<!--
  本文件拥有 frontend 通用模态对话框结构、统一打开关闭动效、关闭行为和基础焦点处理。
  它属于通用组件层，不拥有具体业务表单或 API 调用。
-->
<template>
  <Teleport to="body">
    <Transition name="modal" appear @after-leave="restoreFocus">
      <div
        v-if="open"
        class="modal-layer"
        :class="{
          'modal-layer--nested': nested,
          'modal-layer--network-workspace': networkWorkspace,
        }"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="description ? descriptionId : undefined"
        @mousedown.self="requestClose"
      >
        <section
          ref="panel"
          class="modal-panel"
          :class="{
            'modal-panel--wide': wide,
            'modal-panel--workspace': workspace,
            'modal-panel--network-workspace': networkWorkspace,
            'modal-panel--compact': compact,
          }"
        >
          <header class="modal-header" :class="{ 'modal-header--compact': !description }">
            <div>
              <h2 :id="titleId">{{ title }}</h2>
              <p v-if="description" :id="descriptionId">{{ description }}</p>
            </div>
            <button
              class="icon-button"
              type="button"
              title="关闭"
              aria-label="关闭"
              :disabled="busy"
              @click="requestClose"
            >
              ×
            </button>
          </header>

          <div v-if="$slots.context" class="modal-context">
            <slot name="context" />
          </div>

          <div class="modal-body">
            <slot />
          </div>

          <div v-if="$slots.notice" class="modal-notice">
            <slot name="notice" />
          </div>

          <footer v-if="$slots.actions" class="modal-actions">
            <slot name="actions" />
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from "vue";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { NativeBackPriority } from "../navigation/nativeBack";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    busy?: boolean;
    wide?: boolean;
    workspace?: boolean;
    networkWorkspace?: boolean;
    restoreFocus?: boolean;
    compact?: boolean;
    nested?: boolean;
  }>(),
  {
    description: undefined,
    busy: false,
    wide: false,
    workspace: false,
    networkWorkspace: false,
    restoreFocus: true,
    compact: false,
    nested: false,
  },
);

const emit = defineEmits<{
  close: [];
}>();

const dialogId = useId();
const titleId = `${dialogId}-title`;
const descriptionId = `${dialogId}-description`;
const panel = ref<HTMLElement | null>(null);
let returnFocusElement: HTMLElement | null = null;
let previousBodyOverflow = "";
let previousDocumentOverflow = "";

useNativeBackHandler({
  id: `modal-dialog:${dialogId}`,
  active: () => props.open,
  priority: NativeBackPriority.Dialog,
  handle: () => {
    // registry 已按最近激活顺序选择最上层 Dialog；不等待离场 DOM 从 Transition 中移除。
    if (!props.open) return { handled: false };
    if (props.busy) return { handled: true, reason: "busy-dialog" };
    requestClose();
    return { handled: true, reason: "dialog" };
  },
});

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      window.removeEventListener("keydown", handleKeydown);
      unlockBackgroundScroll();
      return;
    }

    lockBackgroundScroll();
    returnFocusElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    window.addEventListener("keydown", handleKeydown);
    await nextTick();
    const target =
      panel.value?.querySelector<HTMLElement>("[autofocus]") ??
      panel.value?.querySelector<HTMLElement>("input, select, button");
    target?.focus();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
  unlockBackgroundScroll();
});

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === "Tab" && isTopmostDialog()) {
    trapFocus(event);
    return;
  }
  if (event.key === "Escape" && isTopmostDialog()) {
    event.preventDefault();
    requestClose();
  }
}

function trapFocus(event: KeyboardEvent): void {
  const focusable = Array.from(
    panel.value?.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  ).filter((element) => element.getClientRects().length > 0);
  if (!focusable.length) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function lockBackgroundScroll(): void {
  if (!props.networkWorkspace || document.body.style.overflow === "hidden") return;
  previousBodyOverflow = document.body.style.overflow;
  previousDocumentOverflow = document.documentElement.style.overflow;
  document.body.style.overflow = "hidden";
  document.documentElement.style.overflow = "hidden";
}

function unlockBackgroundScroll(): void {
  if (!props.networkWorkspace) return;
  document.body.style.overflow = previousBodyOverflow;
  document.documentElement.style.overflow = previousDocumentOverflow;
}

function isTopmostDialog(): boolean {
  const layers = document.querySelectorAll<HTMLElement>(".modal-layer");
  return panel.value?.closest(".modal-layer") === layers.item(layers.length - 1);
}

function requestClose(): void {
  if (!props.busy) {
    emit("close");
  }
}

/** Dialog 完成离场后再把焦点还给触发控件，避免焦点落到仍在动画中的遮罩下方。 */
function restoreFocus(): void {
  if (props.restoreFocus && returnFocusElement?.isConnected) returnFocusElement.focus();
  returnFocusElement = null;
}
</script>

<style lang="scss" src="./ModalDialog.scss"></style>
