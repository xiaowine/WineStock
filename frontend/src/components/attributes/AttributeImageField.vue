<!-- 本组件拥有图片选择、纯色 PNG 生成、锚定选择浮层和本地预览；上传由所属业务表单在提交阶段统一执行。 -->
<template>
  <div
    ref="fieldRoot"
    class="inbound-file-field"
    :class="{ 'inbound-control--error': invalid }"
    :title="title"
    :aria-label="label"
    :aria-invalid="invalid || undefined"
    tabindex="-1"
  >
    <div
      class="inbound-file-field__preview inbound-file-field__preview--interactive"
      :class="{ 'inbound-file-field__preview--empty': !value }"
    >
      <div
        class="inbound-file-field__preview-main"
        :class="{ 'inbound-file-field__preview-main--with-image': value?.previewUrl }"
      >
        <PreviewImage
          v-if="value?.previewUrl"
          class="inbound-file-field__image-preview"
          :src="value.previewUrl"
          :alt="value.name || label"
        />
        <button
          ref="pickerTrigger"
          class="inbound-file-field__preview-trigger"
          :class="{ 'inbound-file-field__preview-trigger--with-image': value?.previewUrl }"
          type="button"
          :aria-label="value ? `更换图片：${value.name}` : '选择图片'"
          :aria-expanded="pickerOpen"
          :aria-controls="pickerId"
          @click="togglePicker"
        >
          <span
            v-if="!value?.previewUrl"
            class="inbound-file-field__image-placeholder"
            aria-hidden="true"
          >
            <svg v-if="!value" viewBox="0 0 24 24">
              <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
              <circle cx="9" cy="10" r="1.5" />
              <path d="m5.5 17 4.5-4 3.2 2.8 2.3-2 3 3.2" />
            </svg>
            <template v-else>图</template>
          </span>
          <span class="inbound-file-field__preview-copy">
            <strong>{{ value?.name ?? "选择图片" }}</strong>
            <span v-if="!value">本地图片或纯色图片</span>
            <span v-else-if="value.status === 'pending'">将在提交时上传</span>
            <span v-else-if="value.status === 'uploading'">上传中 {{ value.progress }}%</span>
            <span v-else-if="value.status === 'failed'">{{ value.error }}</span>
            <span v-else>{{ formatFileSize(value.sizeBytes) }}</span>
          </span>
        </button>
      </div>
      <button
        v-if="value"
        class="icon-button inbound-file-field__remove"
        type="button"
        title="删除图片"
        aria-label="删除图片"
        @click="remove"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
        </svg>
      </button>
      <span v-else class="inbound-file-field__action-placeholder" aria-hidden="true" />
    </div>

    <input
      ref="fileInput"
      class="inbound-file-field__native-file"
      name="image_file"
      type="file"
      accept="image/png,image/jpeg,image/webp"
      aria-hidden="true"
      tabindex="-1"
      @change="selectFile"
    />

    <Teleport to="body">
      <Transition name="image-picker-popover">
        <div
          v-if="pickerOpen"
          :id="pickerId"
          ref="pickerPopover"
          v-overlay-scrollbar
          class="image-picker-popover"
          :class="{ 'image-picker-popover--above': pickerPlacement === 'above' }"
          :style="pickerStyle"
          role="dialog"
          aria-label="选择图片来源"
        >
          <button class="image-picker-popover__option" type="button" @click="chooseLocalFile">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 16V4M7.5 8.5 12 4l4.5 4.5M5 14v5h14v-5" />
            </svg>
            <span>
              <strong>本地图片</strong>
              <small>PNG、JPEG 或 WebP</small>
            </span>
          </button>
          <button
            class="image-picker-popover__option image-picker-popover__color-option"
            :class="{ 'image-picker-popover__color-option--active': colorPickerOpen }"
            type="button"
            :aria-expanded="colorPickerOpen"
            @click="toggleColorPicker"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M12 3a9 9 0 1 0 0 18h1.2a1.8 1.8 0 0 0 0-3.6h-.7a1.4 1.4 0 0 1 0-2.8H16A5 5 0 0 0 21 9.7C21 6 17 3 12 3Z"
              />
              <circle cx="7.5" cy="10" r=".8" />
              <circle cx="10" cy="6.8" r=".8" />
              <circle cx="14" cy="6.5" r=".8" />
              <circle cx="17" cy="9" r=".8" />
            </svg>
            <span>
              <strong>纯色图片</strong>
              <small>{{ colorPickerOpen ? "调整颜色后立即生成" : "打开颜色选择器" }}</small>
            </span>
            <span
              class="image-picker-popover__color-swatch"
              :style="{ backgroundColor: solidColor }"
              aria-hidden="true"
            />
          </button>
          <AttributeColorPicker
            v-if="colorPickerOpen"
            v-model="solidColor"
            @commit="generateSolidColor"
          />
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, toRef, useId, watch } from "vue";
import { deleteImage, readImage, validateImageFile } from "../../api/files";
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from "../../api/errors";
import { notice } from "../../notices/notice";
import type { ImageDraftValue } from "./imageDraft";
import { useNativeBackHandler } from "../../composables/useNativeBackHandler";
import { NativeBackPriority } from "../../navigation/nativeBack";
import { readSafeAreaInsets } from "../../shell/safeArea";
import PreviewImage from "../PreviewImage.vue";
import {
  createPendingImageDraft,
  createSolidColorImage,
  randomSolidColor,
  releaseImageDraft,
} from "./imageDraft";
import AttributeColorPicker from "./AttributeColorPicker.vue";

const props = withDefaults(
  defineProps<{
    modelValue?: ImageDraftValue;
    invalid?: boolean;
    title?: string;
    label?: string;
    deleteOnRemove?: boolean;
  }>(),
  {
    deleteOnRemove: true,
    label: "图片属性",
  },
);

const emit = defineEmits<{ "update:modelValue": [value: ImageDraftValue | undefined] }>();
const value = toRef(props, "modelValue");
const fieldRoot = ref<HTMLElement | null>(null);
const pickerTrigger = ref<HTMLButtonElement | null>(null);
const pickerPopover = ref<HTMLElement | null>(null);
const fileInput = ref<HTMLInputElement | null>(null);
const pickerOpen = ref(false);
const colorPickerOpen = ref(false);
const pickerPlacement = ref<"above" | "below">("below");
const pickerPosition = ref({ top: 0, left: 0, width: 276 });
const solidColor = ref(randomSolidColor());
const pickerId = `image-picker-${useId()}`;
let solidGenerationSequence = 0;
const pickerStyle = computed(() => ({
  top: `${pickerPosition.value.top}px`,
  left: `${pickerPosition.value.left}px`,
  width: `${pickerPosition.value.width}px`,
}));

useNativeBackHandler({
  id: `image-color-picker:${pickerId}`,
  active: colorPickerOpen,
  priority: NativeBackPriority.TransientOverlay,
  handle: () => {
    if (!colorPickerOpen.value) return { handled: false };
    colorPickerOpen.value = false;
    void nextTick(() => {
      positionPicker();
      pickerPopover.value
        ?.querySelector<HTMLElement>(".image-picker-popover__color-option")
        ?.focus();
    });
    return { handled: true, reason: "transient-overlay" };
  },
});

useNativeBackHandler({
  id: `image-source-picker:${pickerId}`,
  active: pickerOpen,
  priority: NativeBackPriority.TransientOverlay,
  handle: () => {
    if (!pickerOpen.value) return { handled: false };
    closePicker(true);
    return { handled: true, reason: "transient-overlay" };
  },
});

watch(
  value,
  (next, previous) => {
    if (previous && previous !== next) releaseImageDraft(previous);
    if (next?.localFile && !next.previewUrl) next.previewUrl = URL.createObjectURL(next.localFile);
    else if (next?.fileId && !next.previewUrl) void loadPreview(next);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  solidGenerationSequence += 1;
  removePickerListeners();
  const current = value.value;
  releaseImageDraft(current);
  if (current) current.previewUrl = undefined;
});

function togglePicker(): void {
  if (pickerOpen.value) {
    closePicker(true);
    return;
  }
  void openPicker();
}

/** 打开锚定选择浮层并根据触发控件和视口空间决定上下方向。 */
async function openPicker(): Promise<void> {
  colorPickerOpen.value = false;
  pickerOpen.value = true;
  addPickerListeners();
  await nextTick();
  positionPicker();
  pickerPopover.value?.querySelector<HTMLElement>("button, input")?.focus();
}

function closePicker(restoreFocus: boolean): void {
  if (!pickerOpen.value) return;
  pickerOpen.value = false;
  colorPickerOpen.value = false;
  removePickerListeners();
  if (restoreFocus) void nextTick(() => pickerTrigger.value?.focus());
}

/** 使用 fixed 定位避免浮层被编辑器滚动容器裁切，并在空间不足时改为向上展开。 */
function positionPicker(): void {
  const trigger = pickerTrigger.value;
  const popover = pickerPopover.value;
  if (!trigger || !popover) return;
  // edge-to-edge 视口包含系统栏覆盖区域；定位边界必须扣除安全区。
  const inset = readSafeAreaInsets();
  const viewportPadding = 12;
  const gap = 6;
  const triggerRect = trigger.getBoundingClientRect();
  const width = Math.min(276, window.innerWidth - inset.left - inset.right - viewportPadding * 2);
  const left = Math.min(
    Math.max(triggerRect.left, inset.left + viewportPadding),
    window.innerWidth - inset.right - width - viewportPadding,
  );
  const popoverHeight = popover.offsetHeight;
  const belowTop = triggerRect.bottom + gap;
  const aboveTop = triggerRect.top - popoverHeight - gap;
  const useAbove =
    belowTop + popoverHeight > window.innerHeight - inset.bottom - viewportPadding &&
    aboveTop >= inset.top + viewportPadding;
  pickerPlacement.value = useAbove ? "above" : "below";
  pickerPosition.value = {
    top: useAbove ? aboveTop : belowTop,
    left,
    width,
  };
}

async function chooseLocalFile(): Promise<void> {
  closePicker(false);
  await nextTick();
  fileInput.value?.click();
}

async function toggleColorPicker(): Promise<void> {
  colorPickerOpen.value = !colorPickerOpen.value;
  await nextTick();
  positionPicker();
  if (colorPickerOpen.value) {
    pickerPopover.value?.querySelector<HTMLElement>(".attribute-color-picker [tabindex]")?.focus();
  }
}

async function selectFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) {
    pickerTrigger.value?.focus();
    return;
  }
  const error = await validateImageFile(file);
  if (error) {
    notice.warning("无法选择该图片", { detail: error });
    pickerTrigger.value?.focus();
    return;
  }
  replace(file);
  pickerTrigger.value?.focus();
}

function replace(file: File): void {
  const current = value.value;
  releaseImageDraft(current);
  if (current?.fileId && props.deleteOnRemove)
    void deleteImage(current.fileId).catch(() => undefined);
  emit("update:modelValue", createPendingImageDraft(file));
}

async function generateSolidColor(color: string): Promise<void> {
  const sequence = ++solidGenerationSequence;
  try {
    const file = await createSolidColorImage(color);
    if (sequence === solidGenerationSequence) replace(file);
  } catch (error) {
    notice.error(errorMessage(error, "纯色图片生成失败"));
  }
}

async function remove(): Promise<void> {
  closePicker(false);
  const current = value.value;
  if (!current) return;
  releaseImageDraft(current);
  emit("update:modelValue", undefined);
  if (current.fileId && props.deleteOnRemove) {
    try {
      await deleteImage(current.fileId);
    } catch (error) {
      notice.error(errorMessage(error, "删除临时图片失败"));
    }
  }
  await nextTick();
  pickerTrigger.value?.focus();
}

async function loadPreview(target: ImageDraftValue): Promise<void> {
  try {
    target.previewUrl = URL.createObjectURL(await readImage(target.fileId as number));
  } catch (error) {
    target.status = "failed";
    target.error = errorMessage(error, "无法读取已上传图片");
  }
}

function addPickerListeners(): void {
  document.addEventListener("pointerdown", handleOutsidePointerDown);
  document.addEventListener("keydown", handlePickerKeydown);
  document.addEventListener("scroll", handleDocumentScroll, true);
  window.addEventListener("resize", positionPicker);
}

function removePickerListeners(): void {
  document.removeEventListener("pointerdown", handleOutsidePointerDown);
  document.removeEventListener("keydown", handlePickerKeydown);
  document.removeEventListener("scroll", handleDocumentScroll, true);
  window.removeEventListener("resize", positionPicker);
}

function handleOutsidePointerDown(event: PointerEvent): void {
  const target = event.target as Node;
  if (fieldRoot.value?.contains(target) || pickerPopover.value?.contains(target)) return;
  closePicker(false);
}

function handlePickerKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
    closePicker(true);
  }
}

function handleDocumentScroll(): void {
  closePicker(false);
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) return error.message;
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError) return "无法连接到 WineStock 服务";
  if (error instanceof ApiResponseError) return "服务响应格式无效，请检查前后端版本";
  return fallback;
}

function formatFileSize(bytes: number): string {
  if (bytes <= 0) return "已保存图片";
  return bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
</script>

<style scoped>
.inbound-file-field {
  display: grid;
  min-width: 0;
}

.inbound-file-field__preview {
  min-width: 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-text);
}

.inbound-file-field.inbound-control--error .inbound-file-field__preview {
  border-color: var(--color-danger);
}

.inbound-file-field.inbound-control--error:focus-within .inbound-file-field__preview {
  box-shadow: 0 0 0 3px var(--color-danger-ring);
}

.inbound-file-field__preview {
  display: grid;
  min-height: 64px;
  grid-template-columns: minmax(0, 1fr) 36px;
  align-items: center;
  gap: 6px;
  padding: 7px;
}

.inbound-file-field__preview--interactive:hover {
  border-color: var(--color-border-strong);
  background: var(--color-surface-raised);
}

.inbound-file-field__preview-trigger {
  display: grid;
  min-width: 0;
  min-height: 48px;
  grid-template-columns: 48px minmax(0, 1fr);
  align-items: center;
  gap: 9px;
  padding: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--color-text);
  text-align: left;
}

.inbound-file-field__preview-main {
  display: grid;
  min-width: 0;
  grid-template-columns: minmax(0, 1fr);
}

.inbound-file-field__preview-main--with-image {
  grid-template-columns: 48px minmax(0, 1fr);
  align-items: center;
  gap: 9px;
}

.inbound-file-field__image-preview {
  width: 48px;
  height: 48px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
}

.inbound-file-field__preview-trigger--with-image {
  grid-template-columns: minmax(0, 1fr);
}

.inbound-file-field__preview-trigger:focus-visible .inbound-file-field__preview-copy strong {
  color: var(--color-accent);
}

.inbound-file-field__image-placeholder {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
}

.inbound-file-field__image-placeholder svg {
  width: 20px;
  height: 20px;
  fill: none;
  stroke: currentcolor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.inbound-file-field__image-placeholder {
  display: grid;
  place-items: center;
  border: 1px solid var(--color-border);
  background: var(--color-surface-raised);
  color: var(--color-subtle);
  font-size: 12px;
}

.inbound-file-field__preview-copy {
  display: grid;
  min-width: 0;
  gap: 2px;
}

.inbound-file-field__preview-copy strong,
.inbound-file-field__preview-copy > span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.inbound-file-field__preview-copy > span {
  color: var(--color-muted);
  font-size: 11px;
}

.inbound-file-field__remove {
  width: 34px;
  height: 34px;
  border-color: transparent;
  background: transparent;
  color: var(--color-danger);
}

.inbound-file-field__remove:hover {
  background: var(--color-danger-soft);
}

.inbound-file-field__action-placeholder {
  width: 34px;
  height: 34px;
}

.inbound-file-field__remove svg,
.image-picker-popover svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentcolor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.inbound-file-field__native-file {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
  white-space: nowrap;
}

.image-picker-popover {
  position: fixed;
  z-index: var(--z-dialog-popover);
  display: grid;
  max-height: calc(100vh - 24px);
  overflow-y: auto;
  gap: 4px;
  padding: 6px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  box-shadow: var(--shadow-menu);
}

.image-picker-popover__option {
  display: grid;
  width: 100%;
  min-width: 0;
  min-height: 50px;
  grid-template-columns: 28px minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  padding: 6px 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  text-align: left;
}

button.image-picker-popover__option:hover,
.image-picker-popover__color-option:hover {
  background: var(--color-surface-raised);
}

.image-picker-popover__option > svg {
  justify-self: center;
  color: var(--color-muted);
}

.image-picker-popover__option > span {
  display: grid;
  min-width: 0;
  gap: 1px;
}

.image-picker-popover__option strong {
  font-size: 13px;
  font-weight: 650;
}

.image-picker-popover__option small {
  color: var(--color-muted);
  font-size: 11px;
}

.image-picker-popover__color-option {
  cursor: pointer;
}

.image-picker-popover__color-option--active {
  background: var(--color-surface-raised);
}

.image-picker-popover__color-swatch {
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
}

.image-picker-popover-enter-active,
.image-picker-popover-leave-active {
  transition:
    opacity var(--motion-duration-standard) var(--motion-ease-standard),
    transform var(--motion-duration-standard) var(--motion-ease-standard);
  transform-origin: top left;
}

.image-picker-popover--above.image-picker-popover-enter-active,
.image-picker-popover--above.image-picker-popover-leave-active {
  transform-origin: bottom left;
}

.image-picker-popover-enter-from,
.image-picker-popover-leave-to {
  opacity: 0;
  transform: translateY(calc(0px - var(--motion-distance-small))) scale(0.98);
}

.image-picker-popover--above.image-picker-popover-enter-from,
.image-picker-popover--above.image-picker-popover-leave-to {
  transform: translateY(var(--motion-distance-small)) scale(0.98);
}
</style>
