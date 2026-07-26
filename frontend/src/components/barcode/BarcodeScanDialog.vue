<!--
  本组件拥有业务无关的二维码扫描 Dialog：摄像头取流、图片/粘贴降级和识别去重，只回传原文。
  它不解析业务格式、不决定扫码结果的用途，也不拥有解码引擎的加载策略。
-->
<template>
  <ModalDialog
    :open="open"
    :title="title"
    :description="description"
    compact
    :nested="nested"
    @close="emit('close')"
  >
    <div ref="dialogRoot" class="barcode-scan">
      <div
        v-if="cameraSupported"
        class="barcode-scan__viewport"
        :class="{ 'barcode-scan__viewport--flash': flashActive }"
      >
        <video ref="video" playsinline muted></video>
        <canvas ref="overlay" aria-hidden="true"></canvas>
        <div v-if="setupVisible" class="barcode-scan__pending" role="status">
          正在启动摄像头与识别引擎…
        </div>
      </div>

      <p v-if="cameraError" class="barcode-scan__error" role="alert">{{ cameraError }}</p>
      <p v-else-if="!cameraSupported" class="barcode-scan__error" role="alert">
        当前环境无法使用摄像头（常见于局域网 HTTP 访问），请使用下方图片识别。
      </p>

      <div class="barcode-scan__tools">
        <label v-if="selectableCameras.length > 1" class="barcode-scan__camera">
          <span class="visually-hidden">选择摄像头</span>
          <select :value="selectedDeviceId" @change="handleCameraChange">
            <option value="">默认摄像头（后置优先）</option>
            <option
              v-for="(camera, index) in selectableCameras"
              :key="camera.deviceId"
              :value="camera.deviceId"
            >
              {{ camera.label || `摄像头 ${index + 1}` }}
            </option>
          </select>
        </label>
        <button
          v-if="torchSupported"
          class="secondary-button"
          type="button"
          :aria-pressed="torchOn"
          @click="toggleTorch"
        >
          {{ torchOn ? "关闭手电筒" : "打开手电筒" }}
        </button>
        <label class="secondary-button barcode-scan__file">
          {{ cameraSupported ? "从图片识别" : "拍照或选图识别" }}
          <input
            type="file"
            accept="image/*"
            :capture="cameraSupported ? undefined : 'environment'"
            @change="handleFileChange"
          />
        </label>
      </div>

      <p class="barcode-scan__status" role="status" aria-live="polite">
        {{ statusText || internalStatus || "对准二维码即可自动识别，也可拖入图片或 Ctrl+V 粘贴。" }}
      </p>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  decodeQrCameraFrame,
  decodeQrImage,
  loadBarcodeReader,
  type DecodedQrCode,
} from "../../barcode/decoder";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import ModalDialog from "../ModalDialog.vue";

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
    title: "扫码识别",
    description: undefined,
    statusText: "",
    nested: false,
  },
);

const emit = defineEmits<{
  close: [];
  /** 每个新识别到的二维码原文触发一次；相同内容 2 秒内去重。 */
  detect: [text: string];
}>();

const DUPLICATE_WINDOW_MS = 2_000;
/** 单格式快速路径下单帧解码只需几毫秒，用短间隔换更快的对准响应。 */
const FRAME_INTERVAL_MS = 50;
/** 解码前把帧降采样到该宽度：QR 定位不需要全分辨率，像素减少直接加速。 */
const DECODE_MAX_WIDTH = 640;
/** 记住上次使用的摄像头，仓库场景下多摄设备不必每次重选。 */
const CAMERA_DEVICE_STORAGE_KEY = "winestock.barcode.camera-device";

const dialogRoot = ref<HTMLElement | null>(null);
const video = ref<HTMLVideoElement | null>(null);
const overlay = ref<HTMLCanvasElement | null>(null);
const cameraError = ref("");
const internalStatus = ref("");
const torchSupported = ref(false);
const torchOn = ref(false);
const flashActive = ref(false);
const cameras = ref<MediaDeviceInfo[]>([]);
const selectedDeviceId = ref(localStorage.getItem(CAMERA_DEVICE_STORAGE_KEY) ?? "");

const selectableCameras = computed(() => cameras.value.filter((camera) => camera.deviceId));

const cameraSupported =
  window.isSecureContext && typeof navigator.mediaDevices?.getUserMedia === "function";

const setupPending = ref(false);
const setupVisible = useStablePendingIndicator(setupPending, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});

let stream: MediaStream | null = null;
let sessionToken = 0;
let lastText = "";
let lastTextAt = 0;
let flashTimer: ReturnType<typeof setTimeout> | null = null;

const active = computed(() => props.open);

watch(
  () => props.open,
  (open) => {
    if (open) {
      internalStatus.value = "";
      cameraError.value = "";
      lastText = "";
      lastTextAt = 0;
      addPassiveListeners();
      if (cameraSupported) void startCamera();
      return;
    }
    stopCamera();
    removePassiveListeners();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  stopCamera();
  removePassiveListeners();
  if (flashTimer) clearTimeout(flashTimer);
});

async function startCamera(): Promise<void> {
  const token = ++sessionToken;
  setupPending.value = true;
  try {
    const deviceId = selectedDeviceId.value;
    const [, mediaStream] = await Promise.all([loadBarcodeReader(), requestCameraStream(deviceId)]);
    if (token !== sessionToken || !props.open) {
      mediaStream.getTracks().forEach((track) => track.stop());
      return;
    }
    stream = mediaStream;
    const videoElement = video.value;
    if (!videoElement) return;
    videoElement.srcObject = mediaStream;
    await videoElement.play();
    updateTorchAvailability();
    void refreshCameraList();
    void scanLoop(token);
  } catch (error) {
    if (token !== sessionToken) return;
    cameraError.value = cameraErrorMessage(error);
  } finally {
    if (token === sessionToken) setupPending.value = false;
  }
}

/** 记住的设备失效（拔出/权限变化）时自动回退默认摄像头，不让扫码卡死在错误态。 */
async function requestCameraStream(deviceId: string): Promise<MediaStream> {
  const defaultConstraints = {
    video: { facingMode: "environment" as const, width: { ideal: 1280 } },
    audio: false,
  };
  if (!deviceId) return navigator.mediaDevices.getUserMedia(defaultConstraints);
  try {
    return await navigator.mediaDevices.getUserMedia({
      video: { deviceId: { exact: deviceId }, width: { ideal: 1280 } },
      audio: false,
    });
  } catch (error) {
    if (
      error instanceof DOMException &&
      (error.name === "OverconstrainedError" || error.name === "NotFoundError")
    ) {
      selectedDeviceId.value = "";
      localStorage.removeItem(CAMERA_DEVICE_STORAGE_KEY);
      return navigator.mediaDevices.getUserMedia(defaultConstraints);
    }
    throw error;
  }
}

/** 授权成功后刷新设备列表，多摄设备显示切换入口；当前生效设备保持选中。 */
async function refreshCameraList(): Promise<void> {
  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    cameras.value = devices.filter((device) => device.kind === "videoinput");
  } catch {
    cameras.value = [];
  }
  const activeId = stream?.getVideoTracks()[0]?.getSettings().deviceId;
  if (activeId && selectedDeviceId.value && selectedDeviceId.value !== activeId) {
    selectedDeviceId.value = activeId;
  }
}

function handleCameraChange(event: Event): void {
  const deviceId = (event.target as HTMLSelectElement).value;
  selectedDeviceId.value = deviceId;
  if (deviceId) localStorage.setItem(CAMERA_DEVICE_STORAGE_KEY, deviceId);
  else localStorage.removeItem(CAMERA_DEVICE_STORAGE_KEY);
  if (!props.open) return;
  stopCamera();
  cameraError.value = "";
  void startCamera();
}

function stopCamera(): void {
  sessionToken += 1;
  setupPending.value = false;
  stream?.getTracks().forEach((track) => track.stop());
  stream = null;
  torchSupported.value = false;
  torchOn.value = false;
  if (video.value) video.value.srcObject = null;
}

async function scanLoop(token: number): Promise<void> {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return;

  while (token === sessionToken && props.open) {
    const videoElement = video.value;
    if (videoElement && videoElement.videoWidth > 0) {
      const scale = Math.min(1, DECODE_MAX_WIDTH / videoElement.videoWidth);
      canvas.width = Math.round(videoElement.videoWidth * scale);
      canvas.height = Math.round(videoElement.videoHeight * scale);
      context.drawImage(videoElement, 0, 0, canvas.width, canvas.height);
      try {
        const results = await decodeQrCameraFrame(
          context.getImageData(0, 0, canvas.width, canvas.height),
        );
        if (token !== sessionToken) return;
        drawOverlay(results, canvas.width, canvas.height);
        reportResults(results.map((result) => result.text));
      } catch {
        // 单帧解码失败不终止扫描循环；引擎级失败已在启动阶段暴露。
      }
    }
    await new Promise((resolve) => setTimeout(resolve, FRAME_INTERVAL_MS));
  }
}

function drawOverlay(results: DecodedQrCode[], frameWidth: number, frameHeight: number): void {
  const overlayElement = overlay.value;
  if (!overlayElement) return;
  // 检测框坐标基于降采样帧；overlay 使用同一尺寸，由 CSS 拉伸到取景区。
  overlayElement.width = frameWidth;
  overlayElement.height = frameHeight;
  const context = overlayElement.getContext("2d");
  if (!context) return;
  context.clearRect(0, 0, overlayElement.width, overlayElement.height);
  context.strokeStyle = "#4f9b83";
  context.lineWidth = Math.max(3, overlayElement.width / 240);
  for (const { position } of results) {
    context.beginPath();
    context.moveTo(position.topLeft.x, position.topLeft.y);
    context.lineTo(position.topRight.x, position.topRight.y);
    context.lineTo(position.bottomRight.x, position.bottomRight.y);
    context.lineTo(position.bottomLeft.x, position.bottomLeft.y);
    context.closePath();
    context.stroke();
  }
}

/** 去重后向调用方回传结果，并触发震动与取景框闪烁反馈。 */
function reportResults(texts: string[]): void {
  for (const text of texts) {
    const now = performance.now();
    if (text === lastText && now - lastTextAt < DUPLICATE_WINDOW_MS) continue;
    lastText = text;
    lastTextAt = now;
    navigator.vibrate?.(50);
    triggerFlash();
    emit("detect", text);
  }
}

function triggerFlash(): void {
  flashActive.value = true;
  if (flashTimer) clearTimeout(flashTimer);
  flashTimer = setTimeout(() => {
    flashActive.value = false;
    flashTimer = null;
  }, 250);
}

function updateTorchAvailability(): void {
  const track = stream?.getVideoTracks()[0];
  const capabilities = track?.getCapabilities?.() as
    (MediaTrackCapabilities & { torch?: boolean }) | undefined;
  torchSupported.value = Boolean(capabilities?.torch);
}

async function toggleTorch(): Promise<void> {
  const track = stream?.getVideoTracks()[0];
  if (!track) return;
  torchOn.value = !torchOn.value;
  try {
    await track.applyConstraints({
      advanced: [{ torch: torchOn.value } as MediaTrackConstraintSet],
    });
  } catch {
    torchOn.value = false;
    torchSupported.value = false;
  }
}

// ---------- 图片 / 拖放 / 粘贴降级 ----------

function handleFileChange(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (file) void decodeImageSource(file);
}

function handlePaste(event: ClipboardEvent): void {
  if (!active.value || !isTopmostLayer()) return;
  const file = [...(event.clipboardData?.files ?? [])].find((candidate) =>
    candidate.type.startsWith("image/"),
  );
  if (!file) return;
  event.preventDefault();
  void decodeImageSource(file);
}

function handleDragOver(event: DragEvent): void {
  if (!active.value || !isTopmostLayer()) return;
  event.preventDefault();
}

function handleDrop(event: DragEvent): void {
  if (!active.value || !isTopmostLayer()) return;
  event.preventDefault();
  const file = event.dataTransfer?.files[0];
  if (file?.type.startsWith("image/")) void decodeImageSource(file);
}

async function decodeImageSource(file: File): Promise<void> {
  internalStatus.value = "正在识别图片…";
  try {
    const results = await decodeQrImage(file);
    if (results.length === 0) {
      internalStatus.value = "图片中未识别到二维码。";
      return;
    }
    internalStatus.value = "";
    reportResults(results.map((result) => result.text));
  } catch {
    internalStatus.value = "图片识别失败，请重试。";
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

function cameraErrorMessage(error: unknown): string {
  if (error instanceof DOMException) {
    if (error.name === "NotAllowedError") {
      return "摄像头权限被拒绝，可在系统或浏览器设置中重新允许，或使用下方图片识别。";
    }
    if (error.name === "NotFoundError" || error.name === "OverconstrainedError") {
      return "没有找到可用摄像头，请使用下方图片识别。";
    }
    if (error.name === "NotReadableError") {
      return "摄像头被其它应用占用，请关闭后重试或使用下方图片识别。";
    }
  }
  return "摄像头启动失败，请使用下方图片识别。";
}
</script>

<style scoped lang="scss">
.barcode-scan {
  display: grid;
  gap: 10px;
}

.barcode-scan__viewport {
  position: relative;
  overflow: hidden;
  border-radius: var(--radius-md);
  background: #14181d;
  aspect-ratio: 4 / 3;

  video {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  &--flash {
    outline: 3px solid var(--color-teal);
    outline-offset: -3px;
  }
}

.barcode-scan__pending {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgb(20 24 29 / 78%);
  color: #fff;
  font-size: 13px;
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

.barcode-scan__camera select {
  max-width: 220px;
  padding: 7px 9px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 13px;
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
