<!-- 缩略图触发器；可预览时打开 InAppImageViewer。不管理系统栏、不调用 Fullscreen。 -->
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

  <InAppImageViewer
    v-if="previewable && src"
    v-model:open="viewerOpen"
    :src="src"
    :alt="alt"
    :origin-rect="originRect"
    @after-leave="restoreFocus"
  />
</template>

<script setup lang="ts">
import { computed, ref, useAttrs, watch } from "vue";
import InAppImageViewer, { type ImageViewerOriginRect } from "./InAppImageViewer.vue";

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    src?: string;
    alt: string;
    objectFit?: "contain" | "cover" | "fill" | "none" | "scale-down";
    loading?: "eager" | "lazy";
    decoding?: "async" | "auto" | "sync";
    /** 控制是否提供应用内查看；缩略图只读展示时关闭。 */
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
const viewerOpen = ref(false);
const originRect = ref<ImageViewerOriginRect | null>(null);
const imageFailed = ref(false);
const imageUnavailable = computed(() => !props.src || imageFailed.value);
const triggerLabel = computed(() =>
  imageUnavailable.value
    ? `${props.alt} 图片未能加载`
    : props.previewable
      ? `查看图片：${props.alt}`
      : undefined,
);

watch(
  () => props.src,
  () => {
    imageFailed.value = false;
  },
);

function openViewer(): void {
  if (viewerOpen.value || imageUnavailable.value) return;
  const img = trigger.value?.querySelector("img");
  const rect = img?.getBoundingClientRect();
  if (!rect) return;
  originRect.value = {
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height,
    borderRadius: trigger.value ? getComputedStyle(trigger.value).borderRadius : "0",
  };
  viewerOpen.value = true;
}

function restoreFocus(): void {
  originRect.value = null;
  trigger.value?.focus({ preventScroll: true });
}
</script>

<style lang="scss" src="./PreviewImage.scss"></style>
