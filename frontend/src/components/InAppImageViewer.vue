<!--
  应用内图片查看层：固定遮罩 + 安全区避让，不隐藏系统栏、不调用 Fullscreen。
  在 Android 上经 WineStockSystemChrome 切换系统栏为浅色图标，去掉浅色导航栏底。
-->
<template>
  <Teleport to="body">
    <Transition name="in-app-image-viewer" appear @after-leave="onAfterLeave">
      <div
        v-if="open"
        ref="layer"
        class="in-app-image-viewer"
        role="dialog"
        aria-modal="true"
        :aria-label="`查看图片：${alt}`"
        @click.self="close"
      >
        <div class="in-app-image-viewer__dim" aria-hidden="true" />
        <button
          ref="closeButton"
          class="icon-button in-app-image-viewer__close"
          type="button"
          title="关闭图片查看"
          aria-label="关闭图片查看"
          @click="onCloseClick"
          @pointerup="blurCloseButton"
          @touchend="blurCloseButton"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m6 6 12 12M18 6 6 18" />
          </svg>
        </button>
        <img :src="src" :alt="alt" draggable="false" :style="imageStyle" />
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import {
  nextTick,
  onBeforeUnmount,
  ref,
  toRef,
  useId,
  watch,
  type CSSProperties,
} from "vue";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { NativeBackPriority } from "../navigation/nativeBack";

/** 缩略图在视口中的矩形，用于打开/关闭时的共享元素动画。 */
export interface ImageViewerOriginRect {
  left: number;
  top: number;
  width: number;
  height: number;
  borderRadius?: string;
}

const props = defineProps<{
  /** 是否打开查看层。 */
  open: boolean;
  src: string;
  alt: string;
  /** 打开瞬间的缩略图矩形；缺省则仅做淡入淡出。 */
  originRect?: ImageViewerOriginRect | null;
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  /** leave 动画结束，适合恢复触发器焦点。 */
  afterLeave: [];
}>();

const layer = ref<HTMLElement | null>(null);
const closeButton = ref<HTMLButtonElement | null>(null);
const imageStyle = ref<CSSProperties>({});
let previousBodyOverflow = "";
let animationFrame = 0;
let closing = false;

const openRef = toRef(props, "open");

useNativeBackHandler({
  id: `in-app-image-viewer:${useId()}`,
  active: openRef,
  priority: NativeBackPriority.ImagePreview,
  handle: () => {
    if (!props.open) return { handled: false };
    close();
    return { handled: true, reason: "image-preview" };
  },
});

watch(
  () => props.open,
  async (isOpen) => {
    if (isOpen) {
      await present();
      return;
    }
    // 外部把 open 置 false 时（极少）只做清理；正常关闭走 close()。
    teardownChrome();
  },
);

async function present(): Promise<void> {
  closing = false;
  const origin = props.originRect;
  imageStyle.value = origin ? rectStyle(origin) : {};
  previousBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
  // 深色遮罩期间改用浅色系统栏图标，避免 LIGHT_NAVIGATION_BARS 系统浅色底盖住遮罩。
  notifySystemChromeDarkContent(true);
  window.addEventListener("keydown", handleKeydown);
  window.addEventListener("resize", updateExpandedRect);
  await nextTick();
  animationFrame = requestAnimationFrame(() => {
    animationFrame = requestAnimationFrame(() => {
      imageStyle.value = expandedImageStyle();
      closeButton.value?.focus({ preventScroll: true });
    });
  });
}

function onCloseClick(event: MouseEvent): void {
  (event.currentTarget as HTMLElement | null)?.blur();
  close();
}

/** 触控松手后去掉焦点，避免 WebView 粘住 :hover/:focus 高亮。 */
function blurCloseButton(event: Event): void {
  const target = event.currentTarget;
  if (target instanceof HTMLElement) target.blur();
}

function close(): void {
  if (!props.open || closing) return;
  closing = true;
  cancelAnimationFrame(animationFrame);
  const origin = props.originRect;
  if (origin) imageStyle.value = rectStyle(origin);
  teardownChrome();
  animationFrame = requestAnimationFrame(() => {
    emit("update:open", false);
  });
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
}

function teardownChrome(): void {
  window.removeEventListener("keydown", handleKeydown);
  window.removeEventListener("resize", updateExpandedRect);
  document.body.style.overflow = previousBodyOverflow;
  notifySystemChromeDarkContent(false);
}

function onAfterLeave(): void {
  closing = false;
  imageStyle.value = {};
  // 再次确保关闭后恢复浅色栏（防止 teardown 与动画交错）。
  notifySystemChromeDarkContent(false);
  emit("afterLeave");
}

/** Android 宿主钩子；浏览器 / 无接口时静默跳过。 */
function notifySystemChromeDarkContent(enabled: boolean): void {
  try {
    const bridge = (
      window as unknown as {
        WineStockSystemChrome?: { setDarkContent: (value: boolean) => void };
      }
    ).WineStockSystemChrome;
    bridge?.setDarkContent(enabled);
  } catch {
    // 非 Android 或接口不可用
  }
}

function rectStyle(rect: ImageViewerOriginRect): CSSProperties {
  return {
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    borderRadius: rect.borderRadius ?? "0",
  };
}

/** 在安全区 padding 内按原图比例居中，终点与层 padding 一致。 */
function expandedImageStyle(): CSSProperties {
  const layerStyle = layer.value ? getComputedStyle(layer.value) : undefined;
  const fallbackHorizontal = window.innerWidth < 768 ? 12 : 20;
  const topPadding = resolvedCssPixel(layerStyle?.paddingTop, 56);
  const rightPadding = resolvedCssPixel(layerStyle?.paddingRight, fallbackHorizontal);
  const bottomPadding = resolvedCssPixel(layerStyle?.paddingBottom, 20);
  const leftPadding = resolvedCssPixel(layerStyle?.paddingLeft, fallbackHorizontal);
  const availableWidth = Math.max(1, window.innerWidth - leftPadding - rightPadding);
  const availableHeight = Math.max(1, window.innerHeight - topPadding - bottomPadding);

  const img = layer.value?.querySelector("img");
  const naturalWidth = img?.naturalWidth || availableWidth;
  const naturalHeight = img?.naturalHeight || availableHeight;
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

function resolvedCssPixel(value: string | undefined, fallback: number): number {
  const parsed = Number.parseFloat(value ?? "");
  return Number.isFinite(parsed) ? parsed : fallback;
}

function updateExpandedRect(): void {
  if (props.open && !closing) imageStyle.value = expandedImageStyle();
}

onBeforeUnmount(() => {
  cancelAnimationFrame(animationFrame);
  teardownChrome();
});
</script>

<style lang="scss" src="./InAppImageViewer.scss"></style>
