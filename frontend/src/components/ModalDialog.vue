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
        :class="{ 'modal-layer--nested': nested }"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        :aria-describedby="description ? descriptionId : undefined"
        @mousedown.self="requestClose"
      >
        <section
          ref="panel"
          class="modal-panel"
          :class="{ 'modal-panel--wide': wide, 'modal-panel--workspace': workspace, 'modal-panel--compact': compact }"
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
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    busy?: boolean
    wide?: boolean
    workspace?: boolean
    compact?: boolean
    nested?: boolean
  }>(),
  {
    description: undefined,
    busy: false,
    wide: false,
    workspace: false,
    compact: false,
    nested: false,
  },
)

const emit = defineEmits<{
  close: []
}>()

const dialogId = useId()
const titleId = `${dialogId}-title`
const descriptionId = `${dialogId}-description`
const panel = ref<HTMLElement | null>(null)
let returnFocusElement: HTMLElement | null = null

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      window.removeEventListener('keydown', handleKeydown)
      return
    }

    returnFocusElement = document.activeElement instanceof HTMLElement ? document.activeElement : null
    window.addEventListener('keydown', handleKeydown)
    await nextTick()
    const target =
      panel.value?.querySelector<HTMLElement>('[autofocus]') ??
      panel.value?.querySelector<HTMLElement>('input, select, button')
    target?.focus()
  },
  { immediate: true },
)

onBeforeUnmount(() => window.removeEventListener('keydown', handleKeydown))

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && isTopmostDialog()) {
    event.preventDefault()
    requestClose()
  }
}

function isTopmostDialog(): boolean {
  const layers = document.querySelectorAll<HTMLElement>('.modal-layer')
  return panel.value?.closest('.modal-layer') === layers.item(layers.length - 1)
}

function requestClose(): void {
  if (!props.busy) {
    emit('close')
  }
}

/** Dialog 完成离场后再把焦点还给触发控件，避免焦点落到仍在动画中的遮罩下方。 */
function restoreFocus(): void {
  if (returnFocusElement?.isConnected) returnFocusElement.focus()
  returnFocusElement = null
}
</script>

<style lang="scss" src="./ModalDialog.scss"></style>
