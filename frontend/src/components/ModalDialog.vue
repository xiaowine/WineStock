<!--
  本文件拥有 frontend 通用模态对话框结构、关闭行为和基础焦点处理。
  它属于通用组件层，不拥有具体业务表单或 API 调用。
-->
<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="modal-layer"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="titleId"
      :aria-describedby="description ? descriptionId : undefined"
      @mousedown.self="requestClose"
    >
      <section ref="panel" class="modal-panel" :class="{ 'modal-panel--wide': wide }">
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
  }>(),
  {
    description: undefined,
    busy: false,
    wide: false,
  },
)

const emit = defineEmits<{
  close: []
}>()

const dialogId = useId()
const titleId = `${dialogId}-title`
const descriptionId = `${dialogId}-description`
const panel = ref<HTMLElement | null>(null)

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      window.removeEventListener('keydown', handleKeydown)
      return
    }

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
  if (event.key === 'Escape') {
    requestClose()
  }
}

function requestClose(): void {
  if (!props.busy) {
    emit('close')
  }
}
</script>
