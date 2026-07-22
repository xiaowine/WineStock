<!-- 本组件拥有普通图片与全屏查看两种展示状态；它不加载鉴权文件、编辑图片或解释业务含义。 -->
<template>
  <component
    :is="previewable ? 'button' : 'span'"
    ref="trigger"
    v-bind="attrs"
    class="preview-image"
    :class="{ 'preview-image--static': !previewable }"
    :type="previewable ? 'button' : undefined"
    :aria-label="triggerLabel"
    @click.stop="previewable && openViewer()"
  >
    <img
      v-if="!imageUnavailable"
      :src="src"
      :alt="alt"
      :loading="loading"
      :decoding="decoding"
      draggable="false"
      :style="{ objectFit }"
      @error="imageFailed = true"
    />
    <span v-else class="preview-image__fallback" role="img" :aria-label="`${alt} 图片未能加载`">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
        <path d="m6 17 3.8-3.5 2.8 2.5 1.8-1.6 3.6 3.6M8.5 9.5h.01" />
        <path d="m16 8 3 3M19 8l-3 3" />
      </svg>
      <span>图片未能加载</span>
    </span>
  </component>

  <Teleport to="body">
    <Transition name="image-viewer" appear @after-leave="restoreFocus">
      <div
        v-if="viewerOpen"
        ref="viewer"
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
        <img :src="src" :alt="alt" draggable="false" :style="viewerImageStyle" />
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  ref,
  useAttrs,
  useId,
  watch,
  type CSSProperties,
} from "vue";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { NativeBackPriority } from "../navigation/nativeBack";

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    src?: string;
    alt: string;
    objectFit?: "contain" | "cover" | "fill" | "none" | "scale-down";
    loading?: "eager" | "lazy";
    decoding?: "async" | "auto" | "sync";
    /** 控制是否提供全屏查看；缩略图只读展示时关闭，仍复用统一图片渲染。 */
    previewable?: boolean;
  }>(),
  {
    objectFit: "cover",
    loading: "lazy",
    decoding: "async",
    previewable: true,
  },
);

const attrs = useAttrs();
const trigger = ref<HTMLElement | null>(null);
const viewer = ref<HTMLElement | null>(null);
const closeButton = ref<HTMLButtonElement | null>(null);
const viewerOpen = ref(false);
const imageFailed = ref(false);
const imageUnavailable = computed(() => !props.src || imageFailed.value);
const triggerLabel = computed(() =>
  imageUnavailable.value
    ? `${props.alt} 图片未能加载`
    : props.previewable
      ? `全屏查看：${props.alt}`
      : undefined,
);
const viewerImageStyle = ref<CSSProperties>({});
let previousBodyOverflow = "";
let animationFrame = 0;
let closing = false;

useNativeBackHandler({
  id: `image-preview:${useId()}`,
  active: viewerOpen,
  priority: NativeBackPriority.ImagePreview,
  handle: () => {
    if (!viewerOpen.value) return { handled: false };
    closeViewer();
    return { handled: true, reason: "image-preview" };
  },
});

watch(
  () => props.src,
  () => {
    imageFailed.value = false;
  },
);

async function openViewer(): Promise<void> {
  if (viewerOpen.value || imageUnavailable.value) return;
  const sourceRect = sourceImageRect();
  if (!sourceRect) return;
  closing = false;
  viewerImageStyle.value = rectStyle(sourceRect);
  viewerOpen.value = true;
  previousBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
  window.addEventListener("keydown", handleKeydown);
  window.addEventListener("resize", updateExpandedRect);
  await nextTick();
  animationFrame = requestAnimationFrame(() => {
    animationFrame = requestAnimationFrame(() => {
      viewerImageStyle.value = expandedImageStyle();
      closeButton.value?.focus();
    });
  });
}

function closeViewer(): void {
  if (!viewerOpen.value || closing) return;
  closing = true;
  cancelAnimationFrame(animationFrame);
  const sourceRect = sourceImageRect();
  if (sourceRect) viewerImageStyle.value = rectStyle(sourceRect);
  unlockBodyScroll();
  window.removeEventListener("keydown", handleKeydown);
  window.removeEventListener("resize", updateExpandedRect);
  animationFrame = requestAnimationFrame(() => {
    viewerOpen.value = false;
  });
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    closeViewer();
  }
}

function unlockBodyScroll(): void {
  document.body.style.overflow = previousBodyOverflow;
}

function restoreFocus(): void {
  closing = false;
  trigger.value?.focus();
}

function sourceImageRect(): DOMRect | null {
  return trigger.value?.querySelector("img")?.getBoundingClientRect() ?? null;
}

function rectStyle(rect: DOMRect): CSSProperties {
  const borderRadius = trigger.value ? getComputedStyle(trigger.value).borderRadius : "0";
  return {
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    borderRadius,
  };
}

/** 按原图比例计算视口内的最终矩形，保证共享元素动画结束后完整展示图片。 */
function expandedImageStyle(): CSSProperties {
  const sourceImage = trigger.value?.querySelector("img");
  const viewerStyle = viewer.value ? getComputedStyle(viewer.value) : undefined;
  const fallbackHorizontalPadding = window.innerWidth < 768 ? 12 : 20;
  const topPadding = resolvedCssPixel(viewerStyle?.paddingTop, 56);
  const rightPadding = resolvedCssPixel(viewerStyle?.paddingRight, fallbackHorizontalPadding);
  const bottomPadding = resolvedCssPixel(viewerStyle?.paddingBottom, 20);
  const leftPadding = resolvedCssPixel(viewerStyle?.paddingLeft, fallbackHorizontalPadding);
  const availableWidth = Math.max(1, window.innerWidth - leftPadding - rightPadding);
  const availableHeight = Math.max(1, window.innerHeight - topPadding - bottomPadding);
  const sourceRect = sourceImage?.getBoundingClientRect();
  const naturalWidth = sourceImage?.naturalWidth || sourceRect?.width || 1;
  const naturalHeight = sourceImage?.naturalHeight || sourceRect?.height || 1;
  const scale = Math.min(availableWidth / naturalWidth, availableHeight / naturalHeight);
  const width = naturalWidth * scale;
  const height = naturalHeight * scale;
  return {
    left: `${leftPadding + (availableWidth - width) / 2}px`,
    top: `${topPadding + (availableHeight - height) / 2}px`,
    width: `${width}px`,
    height: `${height}px`,
    borderRadius: "0px",
  };
}

/** 读取浏览器已解析的 CSS px，使图片终点与 env/shell 合并后的实际安全区保持一致。 */
function resolvedCssPixel(value: string | undefined, fallback: number): number {
  const parsed = Number.parseFloat(value ?? "");
  return Number.isFinite(parsed) ? parsed : fallback;
}

function updateExpandedRect(): void {
  if (viewerOpen.value && !closing) viewerImageStyle.value = expandedImageStyle();
}

onBeforeUnmount(() => {
  cancelAnimationFrame(animationFrame);
  window.removeEventListener("keydown", handleKeydown);
  window.removeEventListener("resize", updateExpandedRect);
  if (viewerOpen.value) unlockBodyScroll();
});
</script>

<style lang="scss" src="./PreviewImage.scss"></style>
