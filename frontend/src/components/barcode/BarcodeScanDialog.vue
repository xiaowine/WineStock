<!--
  本组件拥有业务无关的二维码扫描 Dialog 编排：状态行、工具栏、图片/拖放/粘贴降级与识别去重，只回传原文。
  摄像头取景与点击对焦由 BarcodeCameraView 拥有；它不解析业务格式，也不决定扫码结果的用途。
-->
<template>
  <ModalDialog
    :open="open"
    :title="title ?? t('misc.barcodeScanTitle')"
    :description="description"
    compact
    :nested="nested"
    @close="emit('close')"
  >
    <div ref="dialogRoot" class="barcode-scan">
      <!-- 提示固定在取景画面上方：底部对齐面板中，上方文字变化不会挪动画面及其下方内容。 -->
      <p
        v-if="cameraSupported"
        class="barcode-scan__status-line"
        :class="{ 'barcode-scan__status-line--error': Boolean(cameraError) }"
        :role="cameraError ? 'alert' : 'status'"
        aria-live="polite"
      >
        {{ viewportStatusText }}
      </p>
      <BarcodeCameraView
        v-if="cameraSupported"
        ref="cameraView"
        :active="open"
        @detect="emit('detect', $event)"
        @error="handleCameraError"
        @hint="internalStatus = $event"
      />

      <p v-if="!cameraSupported" class="barcode-scan__error" role="alert">
        {{ $t("misc.barcodeCameraUnavailable") }}
      </p>

      <div class="barcode-scan__tools">
        <button
          v-if="cameraView?.multiCamera"
          class="icon-button"
          type="button"
          :title="$t('components.barcodeCameraSwitch')"
          :aria-label="$t('components.barcodeCameraSwitch')"
          @click="handleCycleCamera($event)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M3.5 7.5h4l2-2.5h5l2 2.5h4v12h-17z" />
            <path d="M9.3 14.7a3 3 0 0 1 5-2.9" />
            <path d="M14.7 12.3a3 3 0 0 1-5 2.9" />
            <path d="m14.9 9.9-.2 1.9-1.9-.4" />
            <path d="m9.1 17.1.2-1.9 1.9.4" />
          </svg>
        </button>
        <button
          v-if="cameraView?.torchSupported"
          class="icon-button"
          :class="{ 'barcode-scan__tool--active': cameraView?.torchOn }"
          type="button"
          :title="cameraView?.torchOn ? $t('misc.barcodeTorchOff') : $t('misc.barcodeTorchOn')"
          :aria-label="cameraView?.torchOn ? $t('misc.barcodeTorchOff') : $t('misc.barcodeTorchOn')"
          :aria-pressed="cameraView?.torchOn"
          @click="handleToggleTorch($event)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M9 3h6v3l-1.5 2.5v11a1.5 1.5 0 0 1-3 0v-11L9 6z" />
            <path d="M9 6h6" />
            <path d="M12 11.5v3" />
          </svg>
        </button>
        <label class="secondary-button barcode-scan__file">
          {{ cameraSupported ? $t("misc.barcodeRecognizeFromImage") : $t("misc.barcodeCaptureOrSelect") }}
          <input
            type="file"
            accept="image/*"
            :capture="cameraSupported ? undefined : 'environment'"
            @change="handleFileChange"
          />
        </label>
      </div>

      <p v-if="!cameraSupported" class="barcode-scan__status" role="status" aria-live="polite">
        {{ statusText || internalStatus || $t("misc.barcodeDropOrPasteHint") }}
      </p>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { decodeQrImage } from "../../barcode/decoder";
import { trackTelemetryIssue } from "../../telemetry/clarity";
import ModalDialog from "../ModalDialog.vue";
import BarcodeCameraView from "./BarcodeCameraView.vue";

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    open: boolean;
    title?: string;
    description?: string;
    /** 由调用方驱动的业务反馈行，例如「已添加 C2687125」。 */
    statusText?: string;
    /** 在既有 Dialog 之上打开时使用嵌套层级视觉。 */
    nested?: boolean;
  }>(),
  {
    title: undefined,
    description: undefined,
    statusText: "",
    nested: false,
  },
);

const emit = defineEmits<{
  close: [];
  /** 每个新识别到的二维码原文触发一次；相机侧相同内容 2 秒内去重。 */
  detect: [text: string];
}>();

const dialogRoot = ref<HTMLElement | null>(null);
const cameraView = ref<InstanceType<typeof BarcodeCameraView> | null>(null);
const cameraError = ref("");
const internalStatus = ref("");

const cameraSupported =
  window.isSecureContext && typeof navigator.mediaDevices?.getUserMedia === "function";

const viewportStatusText = computed(
  () =>
    cameraError.value ||
    props.statusText ||
    internalStatus.value ||
    t("misc.barcodeViewportHint"),
);

watch(
  () => props.open,
  (open) => {
    if (open) {
      internalStatus.value = "";
      cameraError.value = "";
      addPassiveListeners();
      return;
    }
    removePassiveListeners();
  },
  { immediate: true },
);

/** 摄像头初始化/取流失败几乎无法本地复现，记排查事件；同一次打开只记首个错误。 */
function handleCameraError(message: string): void {
  if (!cameraError.value && message) trackTelemetryIssue("scan_camera_error");
  cameraError.value = message;
}

function handleCycleCamera(event: MouseEvent): void {
  (event.currentTarget as HTMLElement | null)?.blur();
  cameraView.value?.cycleCamera();
}

function handleToggleTorch(event: MouseEvent): void {
  (event.currentTarget as HTMLElement | null)?.blur();
  void cameraView.value?.toggleTorch();
}

// ---------- 图片 / 拖放 / 粘贴降级 ----------

function handleFileChange(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (file) void decodeImageSource(file);
}

function handlePaste(event: ClipboardEvent): void {
  if (!props.open || !isTopmostLayer()) return;
  const file = [...(event.clipboardData?.files ?? [])].find((candidate) =>
    candidate.type.startsWith("image/"),
  );
  if (!file) return;
  event.preventDefault();
  void decodeImageSource(file);
}

function handleDragOver(event: DragEvent): void {
  if (!props.open || !isTopmostLayer()) return;
  event.preventDefault();
}

function handleDrop(event: DragEvent): void {
  if (!props.open || !isTopmostLayer()) return;
  event.preventDefault();
  const file = event.dataTransfer?.files[0];
  if (file?.type.startsWith("image/")) void decodeImageSource(file);
}

async function decodeImageSource(file: File): Promise<void> {
  internalStatus.value = t("misc.barcodeRecognizingImage");
  try {
    const results = await decodeQrImage(file);
    if (results.length === 0) {
      internalStatus.value = t("misc.barcodeImageNoQr");
      return;
    }
    internalStatus.value = "";
    navigator.vibrate?.(50);
    for (const result of results) emit("detect", result.text);
  } catch {
    internalStatus.value = t("misc.barcodeImageFailed");
  }
}

function addPassiveListeners(): void {
  window.addEventListener("paste", handlePaste);
  window.addEventListener("dragover", handleDragOver);
  window.addEventListener("drop", handleDrop);
}

function removePassiveListeners(): void {
  window.removeEventListener("paste", handlePaste);
  window.removeEventListener("dragover", handleDragOver);
  window.removeEventListener("drop", handleDrop);
}

/** 扫码层必须是最上层 modal 才响应粘贴与拖放，避免被再叠 Dialog 时误收输入。 */
function isTopmostLayer(): boolean {
  const layers = document.querySelectorAll(".modal-layer");
  const ownLayer = dialogRoot.value?.closest(".modal-layer") ?? null;
  return ownLayer !== null && ownLayer === layers.item(layers.length - 1);
}
</script>

<style scoped lang="scss">
.barcode-scan {
  display: grid;
  gap: 10px;
}

/* 预留稳定单行高度，短文案变化不改变提示条尺寸；极端换行由下方内容位置不变兜底。 */
.barcode-scan__status-line {
  margin: 0;
  min-height: 20px;
  color: var(--color-muted);
  font-size: 13px;
  line-height: 1.5;
}

.barcode-scan__status-line--error {
  color: var(--color-danger);
}

.barcode-scan__error {
  margin: 0;
  color: var(--color-danger);
  font-size: 13px;
}

.barcode-scan__tools {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.barcode-scan__tool--active {
  border-color: var(--color-accent);
  background: var(--color-accent-soft);
  color: var(--color-accent);
}

.barcode-scan__file {
  cursor: pointer;

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    clip-path: inset(50%);
    white-space: nowrap;
  }
}

.barcode-scan__status {
  margin: 0;
  min-height: 20px;
  color: var(--color-muted);
  font-size: 13px;
}
</style>
