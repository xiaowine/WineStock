<!-- 本组件拥有普通图片与全屏查看两种展示状态；它不加载鉴权文件、编辑图片或解释业务含义。 -->
<template>
  <button
    ref="trigger"
    v-bind="attrs"
    class="preview-image"
    type="button"
    :aria-label="`全屏查看：${alt}`"
    @click.stop="openViewer"
  >
    <img
      :src="src"
      :alt="alt"
      :loading="loading"
      :decoding="decoding"
      draggable="false"
      :style="{ objectFit }"
    />
  </button>

  <Teleport to="body">
    <Transition name="image-viewer" appear @after-leave="restoreFocus">
      <div
        v-if="viewerOpen"
        class="image-viewer"
        role="dialog"
        aria-modal="true"
        :aria-label="`查看图片：${alt}`"
        @click.self="closeViewer"
      >
        <button
          ref="closeButton"
          class="icon-button image-viewer__close"
          type="button"
          title="关闭图片查看"
          aria-label="关闭图片查看"
          @click="closeViewer"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m6 6 12 12M18 6 6 18" />
          </svg>
        </button>
        <img
          :src="src"
          :alt="alt"
          draggable="false"
          :style="viewerImageStyle"
        />
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useAttrs, type CSSProperties } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  src: string
  alt: string
  objectFit?: 'contain' | 'cover' | 'fill' | 'none' | 'scale-down'
  loading?: 'eager' | 'lazy'
  decoding?: 'async' | 'auto' | 'sync'
}>(), {
  objectFit: 'cover',
  loading: 'lazy',
  decoding: 'async',
})

const attrs = useAttrs()
const trigger = ref<HTMLButtonElement | null>(null)
const closeButton = ref<HTMLButtonElement | null>(null)
const viewerOpen = ref(false)
const viewerImageStyle = ref<CSSProperties>({})
let previousBodyOverflow = ''
let animationFrame = 0
let closing = false

async function openViewer(): Promise<void> {
  if (viewerOpen.value) return
  const sourceRect = sourceImageRect()
  if (!sourceRect) return
  closing = false
  viewerImageStyle.value = rectStyle(sourceRect)
  viewerOpen.value = true
  previousBodyOverflow = document.body.style.overflow
  document.body.style.overflow = 'hidden'
  window.addEventListener('keydown', handleKeydown)
  window.addEventListener('resize', updateExpandedRect)
  await nextTick()
  animationFrame = requestAnimationFrame(() => {
    animationFrame = requestAnimationFrame(() => {
      viewerImageStyle.value = expandedImageStyle()
      closeButton.value?.focus()
    })
  })
}

function closeViewer(): void {
  if (!viewerOpen.value || closing) return
  closing = true
  cancelAnimationFrame(animationFrame)
  const sourceRect = sourceImageRect()
  if (sourceRect) viewerImageStyle.value = rectStyle(sourceRect)
  unlockBodyScroll()
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('resize', updateExpandedRect)
  animationFrame = requestAnimationFrame(() => {
    viewerOpen.value = false
  })
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault()
    closeViewer()
  }
}

function unlockBodyScroll(): void {
  document.body.style.overflow = previousBodyOverflow
}

function restoreFocus(): void {
  closing = false
  trigger.value?.focus()
}

function sourceImageRect(): DOMRect | null {
  return trigger.value?.querySelector('img')?.getBoundingClientRect() ?? null
}

function rectStyle(rect: DOMRect): CSSProperties {
  const borderRadius = trigger.value ? getComputedStyle(trigger.value).borderRadius : '0'
  return {
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    borderRadius,
  }
}

/** 按原图比例计算视口内的最终矩形，保证共享元素动画结束后完整展示图片。 */
function expandedImageStyle(): CSSProperties {
  const sourceImage = trigger.value?.querySelector('img')
  const horizontalPadding = window.innerWidth < 768 ? 12 : 20
  const topPadding = 56
  const bottomPadding = 20
  const availableWidth = Math.max(1, window.innerWidth - horizontalPadding * 2)
  const availableHeight = Math.max(1, window.innerHeight - topPadding - bottomPadding)
  const sourceRect = sourceImage?.getBoundingClientRect()
  const naturalWidth = sourceImage?.naturalWidth || sourceRect?.width || 1
  const naturalHeight = sourceImage?.naturalHeight || sourceRect?.height || 1
  const scale = Math.min(availableWidth / naturalWidth, availableHeight / naturalHeight)
  const width = naturalWidth * scale
  const height = naturalHeight * scale
  return {
    left: `${(window.innerWidth - width) / 2}px`,
    top: `${topPadding + (availableHeight - height) / 2}px`,
    width: `${width}px`,
    height: `${height}px`,
    borderRadius: '0px',
  }
}

function updateExpandedRect(): void {
  if (viewerOpen.value && !closing) viewerImageStyle.value = expandedImageStyle()
}

onBeforeUnmount(() => {
  cancelAnimationFrame(animationFrame)
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('resize', updateExpandedRect)
  if (viewerOpen.value) unlockBodyScroll()
})
</script>

<style lang="scss" src="./PreviewImage.scss"></style>
