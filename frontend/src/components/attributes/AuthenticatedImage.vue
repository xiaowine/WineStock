<!-- 本组件通过鉴权文件接口加载只读图片并管理 Blob URL；可按调用方要求组合全屏预览，但不编辑、上传或绑定文件。 -->
<template>
  <div
    class="authenticated-image"
    :class="{ 'authenticated-image--loading': loading }"
    :style="sizeStyle"
  >
    <PreviewImage v-if="!loading" :src="previewUrl" :alt="alt" :previewable="previewable" />
    <span v-else aria-hidden="true">…</span>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
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
const previewUrl = ref("");
const loading = ref(false);
const sizeStyle = computed(() => ({ width: `${props.size}px`, height: `${props.size}px` }));
let controller: AbortController | null = null;

watch(() => props.fileId, load, { immediate: true });
onBeforeUnmount(clear);

/** 读取新的受控文件并替换旧 Blob URL；取消或失败时保留紧凑占位。 */
async function load(): Promise<void> {
  clear();
  controller = new AbortController();
  loading.value = true;
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
