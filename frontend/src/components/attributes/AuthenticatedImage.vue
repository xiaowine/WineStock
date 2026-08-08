<!-- 本组件通过鉴权文件接口读取只读图片并管理 Blob URL；图片接近视口才拉取，可按调用方要求组合全屏预览，但不编辑、上传或绑定文件。 -->
<template>
  <div
    ref="root"
    class="authenticated-image"
    :class="{ 'authenticated-image--loading': loading }"
    :style="sizeStyle"
  >
    <PreviewImage v-if="!loading" :src="previewUrl" :alt="alt" :previewable="previewable" />
    <span v-else aria-hidden="true">…</span>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { readImage } from "../../api/files";
import PreviewImage from "../PreviewImage.vue";

const props = withDefaults(
  defineProps<{
    fileId: number;
    alt: string;
    size?: number;
    previewable?: boolean;
  }>(),
  {
    size: 44,
    previewable: true,
  },
);
const root = ref<HTMLElement | null>(null);
const previewUrl = ref("");
const loading = ref(false);
const sizeStyle = computed(() => ({ width: `${props.size}px`, height: `${props.size}px` }));
let controller: AbortController | null = null;
let observed = false;

// 共享的视口观察器：图片接近视口才开始拉取，避免物品列表一次性拉取全部图片。
// 语义对齐 Blink 原生 loading="lazy"，但鉴权图片必须先用 JS 拉取 Blob，
// 原生属性无法延后 fetch 本身，因此在这里手动门控。
let sharedObserver: IntersectionObserver | null = null;
const intersectCallbacks = new WeakMap<Element, () => void>();

watch(() => props.fileId, startPending, { immediate: true });
onMounted(startPending);
onBeforeUnmount(stopObserving);
onBeforeUnmount(clear);

function getSharedObserver(): IntersectionObserver | null {
  if (typeof IntersectionObserver === "undefined") return null;
  if (!sharedObserver) {
    sharedObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          sharedObserver?.unobserve(entry.target);
          const callback = intersectCallbacks.get(entry.target);
          intersectCallbacks.delete(entry.target);
          callback?.();
        }
      },
      { rootMargin: "500px" },
    );
  }
  return sharedObserver;
}

/** 进入观察等待：展示紧凑占位，直到宿主接近视口才真正拉取；观察器不可用时退化为立即拉取。 */
function startPending(): void {
  clear();
  loading.value = true;
  if (observed) stopObserving();
  const element = root.value;
  if (!element) return;
  const observer = getSharedObserver();
  if (!observer) {
    void load();
    return;
  }
  observed = true;
  intersectCallbacks.set(element, () => void load());
  observer.observe(element);
}

function stopObserving(): void {
  if (!observed || !root.value) return;
  sharedObserver?.unobserve(root.value);
  intersectCallbacks.delete(root.value);
  observed = false;
}

/** 读取新的受控文件并替换旧 Blob URL；取消或失败时保留紧凑占位。 */
async function load(): Promise<void> {
  clear();
  controller = new AbortController();
  try {
    previewUrl.value = URL.createObjectURL(await readImage(props.fileId, controller.signal));
  } catch (error) {
    if (!(error instanceof DOMException && error.name === "AbortError")) previewUrl.value = "";
  } finally {
    controller = null;
    loading.value = false;
  }
}

function clear(): void {
  controller?.abort();
  controller = null;
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value);
  previewUrl.value = "";
}
</script>

<style scoped>
.authenticated-image {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 9px;
  background: var(--color-surface);
  color: var(--color-subtle);
}
.authenticated-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.authenticated-image--loading {
  background: var(--color-surface-raised);
}
</style>
