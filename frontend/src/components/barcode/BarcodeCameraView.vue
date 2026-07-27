<!--
  本组件拥有扫码取景区：摄像头会话、逐帧解码、检测框与识别反馈、设备循环切换、手电筒和点击对焦。
  它按去重后的原文向外回传识别结果，不解析业务格式，也不拥有 Dialog 结构或图片降级路径。
-->
<template>
  <div
    ref="viewportElement"
    class="barcode-camera"
    :class="{ 'barcode-camera--flash': flashActive }"
    @click="handleViewportClick"
  >
    <!-- 首帧就绪前隐藏 video，避免 WebView 渲染引擎默认的媒体加载占位；取景区自身的深色底即中性背景。 -->
    <video
      ref="video"
      class="barcode-camera__video"
      :class="{ 'barcode-camera__video--live': cameraLive }"
      playsinline
      muted
      @playing="handleCameraPlaying"
    ></video>
    <canvas ref="overlay" aria-hidden="true"></canvas>
    <span
      v-if="focusRing"
      :key="focusRing.key"
      class="barcode-camera__focus-ring"
      :style="{ left: `${focusRing.x}px`, top: `${focusRing.y}px` }"
      aria-hidden="true"
      @animationend="focusRing = null"
    />
    <div v-if="setupVisible" class="barcode-camera__pending" role="status">
      正在启动摄像头与识别引擎…
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { decodeQrCameraFrame, loadBarcodeReader, type DecodedQrCode } from "../../barcode/decoder";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";

/** mediacapture-image 扩展能力；TS 内置 DOM 类型尚未收录，按 Chromium 实现声明。 */
type CameraTrackCapabilities = MediaTrackCapabilities & {
  torch?: boolean;
  focusMode?: string[];
};

const props = defineProps<{
  /** 取景是否活跃；关闭时停止取流并复位状态。 */
  active: boolean;
}>();

const emit = defineEmits<{
  /** 每个新识别到的二维码原文触发一次；相同内容 2 秒内去重。 */
  detect: [text: string];
  /** 摄像头错误文案；成功启动时以空串清除。 */
  error: [message: string];
  /** 面向状态行的轻量提示，例如切换后的摄像头名称。 */
  hint: [message: string];
}>();

const DUPLICATE_WINDOW_MS = 2_000;
/** 单格式快速路径下单帧解码只需几毫秒，用短间隔换更快的对准响应。 */
const FRAME_INTERVAL_MS = 50;
/** 解码前把帧降采样到该宽度：QR 定位不需要全分辨率，像素减少直接加速。 */
const DECODE_MAX_WIDTH = 640;
/** 记住上次使用的摄像头，仓库场景下多摄设备不必每次重选。 */
const CAMERA_DEVICE_STORAGE_KEY = "winestock.barcode.camera-device";
/** manual→continuous 往返之间的短暂停留，确保模式切换真实下发到相机。 */
const FOCUS_KICK_HOLD_MS = 120;

const viewportElement = ref<HTMLElement | null>(null);
const video = ref<HTMLVideoElement | null>(null);
const overlay = ref<HTMLCanvasElement | null>(null);
const cameraLive = ref(false);
const flashActive = ref(false);
const torchSupported = ref(false);
const torchOn = ref(false);
const focusCapable = ref(false);
const focusRing = ref<{ x: number; y: number; key: number } | null>(null);
const cameras = ref<MediaDeviceInfo[]>([]);
const selectedDeviceId = ref(localStorage.getItem(CAMERA_DEVICE_STORAGE_KEY) ?? "");

const selectableCameras = computed(() => cameras.value.filter((camera) => camera.deviceId));
const multiCamera = computed(() => selectableCameras.value.length > 1);

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
let focusRingKey = 0;

watch(
  () => props.active,
  (active) => {
    if (active) {
      lastText = "";
      lastTextAt = 0;
      void startCamera();
      return;
    }
    stopCamera();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  stopCamera();
  if (flashTimer) clearTimeout(flashTimer);
});

defineExpose({
  /** 是否有多个可切换摄像头（授权成功后才可知）。 */
  multiCamera,
  torchSupported,
  torchOn,
  cycleCamera,
  toggleTorch,
});

async function startCamera(): Promise<void> {
  const token = ++sessionToken;
  setupPending.value = true;
  try {
    const [, mediaStream] = await Promise.all([
      loadBarcodeReader(),
      requestCameraStream(selectedDeviceId.value),
    ]);
    if (token !== sessionToken || !props.active) {
      mediaStream.getTracks().forEach((track) => track.stop());
      return;
    }
    stream = mediaStream;
    const videoElement = video.value;
    if (!videoElement) return;
    videoElement.srcObject = mediaStream;
    await videoElement.play();
    readTrackCapabilities();
    emit("error", "");
    void refreshCameraList();
    void scanLoop(token);
  } catch (error) {
    if (token !== sessionToken) return;
    emit("error", cameraErrorMessage(error));
  } finally {
    if (token === sessionToken) setupPending.value = false;
  }
}

function stopCamera(): void {
  sessionToken += 1;
  setupPending.value = false;
  cameraLive.value = false;
  const videoElement = video.value;
  // WebView 的视频合成层可能早于 Vue class 更新绘制空 srcObject；先同步隐藏，避免闪出原生占位。
  if (videoElement) videoElement.style.visibility = "hidden";
  stream?.getTracks().forEach((track) => track.stop());
  stream = null;
  torchSupported.value = false;
  torchOn.value = false;
  focusCapable.value = false;
  focusRing.value = null;
  if (videoElement) videoElement.srcObject = null;
}

/** 新流真正产出画面且 live class 已落到 DOM 后，再显示视频合成表面。 */
async function handleCameraPlaying(): Promise<void> {
  const videoElement = video.value;
  const playingStream = stream;
  if (
    !videoElement ||
    !playingStream ||
    !props.active ||
    videoElement.srcObject !== playingStream
  ) {
    return;
  }
  cameraLive.value = true;
  await nextTick();
  if (props.active && stream === playingStream && videoElement.srcObject === playingStream) {
    videoElement.style.removeProperty("visibility");
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

/** 读取当前 track 的扩展能力：手电筒与对焦控制支持。 */
function readTrackCapabilities(): void {
  const track = stream?.getVideoTracks()[0];
  const capabilities = track?.getCapabilities?.() as CameraTrackCapabilities | undefined;
  torchSupported.value = Boolean(capabilities?.torch);
  focusCapable.value = Boolean(
    capabilities?.focusMode?.includes("continuous") && capabilities.focusMode.includes("manual"),
  );
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

/** 在可用摄像头间循环切换：以当前生效设备为基准取下一个，记住选择并重启取流。 */
function cycleCamera(): void {
  const availableCameras = selectableCameras.value;
  if (availableCameras.length < 2) return;
  const activeId = stream?.getVideoTracks()[0]?.getSettings().deviceId ?? selectedDeviceId.value;
  const activeIndex = availableCameras.findIndex((camera) => camera.deviceId === activeId);
  const nextIndex = (activeIndex + 1) % availableCameras.length;
  const next = availableCameras[nextIndex];
  selectedDeviceId.value = next.deviceId;
  localStorage.setItem(CAMERA_DEVICE_STORAGE_KEY, next.deviceId);
  emit("hint", `已切换到 ${next.label || `摄像头 ${nextIndex + 1}`}`);
  if (!props.active) return;
  stopCamera();
  void startCamera();
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

/**
 * 点击取景区触发一次全局重新对焦：manual（保持当前距离）→ continuous 往返，
 * 强制连续对焦算法重新收敛。裸 single-shot 在部分 WebView 上是静默空操作，
 * 而 Web API 的区域对焦参数（pointsOfInterest）Chromium 未实现，位置仅用于对焦框动效。
 */
async function handleViewportClick(event: MouseEvent): Promise<void> {
  if (!focusCapable.value) return;
  const track = stream?.getVideoTracks()[0];
  const viewport = viewportElement.value;
  if (!track || !viewport) return;
  const rect = viewport.getBoundingClientRect();
  focusRing.value = {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
    key: ++focusRingKey,
  };
  const settings = track.getSettings() as MediaTrackSettings & { focusDistance?: number };
  try {
    await track.applyConstraints({
      advanced: [
        {
          focusMode: "manual",
          ...(settings.focusDistance !== undefined
            ? { focusDistance: settings.focusDistance }
            : {}),
        } as MediaTrackConstraintSet,
      ],
    });
    await new Promise((resolve) => setTimeout(resolve, FOCUS_KICK_HOLD_MS));
  } catch {
    // 切换失败也继续尝试恢复连续对焦，避免留在不确定状态。
  }
  await track
    .applyConstraints({ advanced: [{ focusMode: "continuous" } as MediaTrackConstraintSet] })
    .catch(() => undefined);
}

async function scanLoop(token: number): Promise<void> {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return;

  while (token === sessionToken && props.active) {
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
.barcode-camera {
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

.barcode-camera__video {
  opacity: 0;
  transition: opacity var(--motion-duration-standard) var(--motion-ease-standard);
}

.barcode-camera__video--live {
  opacity: 1;
}

.barcode-camera__pending {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgb(20 24 29 / 78%);
  color: #fff;
  font-size: 13px;
}

.barcode-camera__focus-ring {
  position: absolute;
  width: 56px;
  height: 56px;
  border: 2px solid #fff;
  border-radius: var(--radius-sm);
  box-shadow: 0 0 0 1px rgb(20 24 29 / 45%);
  pointer-events: none;
  transform: translate(-50%, -50%);
  animation: barcode-camera-focus 0.7s var(--motion-ease-standard) forwards;
}

@keyframes barcode-camera-focus {
  0% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(1.35);
  }

  25% {
    opacity: 1;
  }

  70% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1);
  }

  100% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .barcode-camera__focus-ring {
    animation-duration: 0.4s;
  }

  .barcode-camera__video {
    transition: none;
  }
}
</style>
