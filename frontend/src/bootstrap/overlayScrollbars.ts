// 本文件拥有全局浮层滚动条，属于 frontend 启动层；它不改变业务滚动容器的所有权。
// 滑块挂在 Dialog 之上的固定层以保证可见；嵌套 Dialog 打开时隐藏被遮挡宿主的滑块，避免下层滑块穿透。

type ScrollAxis = "vertical" | "horizontal";

interface ThumbGeometry {
  maxScroll: number;
  travel: number;
}

interface OverlayScrollbarEntry {
  element: HTMLElement;
  horizontalGeometry?: ThumbGeometry;
  horizontalThumb?: HTMLDivElement;
  verticalGeometry?: ThumbGeometry;
  verticalThumb?: HTMLDivElement;
}

interface ScrollableAxes {
  horizontal: boolean;
  vertical: boolean;
}

interface VisibleRect {
  bottom: number;
  height: number;
  left: number;
  right: number;
  top: number;
  width: number;
}

const ACTIVE_MEDIA_QUERY = "all";
const ROOT_ID = "app-overlay-scrollbars";
const TRACK_INSET = 3;
const THUMB_HIT_SIZE = 12;
const MIN_THUMB_LENGTH = 32;
const SCROLLABLE_OVERFLOW = new Set(["auto", "scroll"]);

let activeMediaQuery: MediaQueryList | undefined;
let overlayRoot: HTMLDivElement | undefined;
let refreshFrame = 0;
let resizeObserver: ResizeObserver | undefined;
const entries = new Map<HTMLElement, OverlayScrollbarEntry>();
const registeredElements = new Set<HTMLElement>();
// MutationObserver 只能整组 disconnect，按宿主各建一个以便单独解绑。
const contentObservers = new WeakMap<HTMLElement, MutationObserver>();

/**
 * 宿主内部影响布局的内容变更：新增/移除节点、文本替换，以及 style/class 切换。
 * 高度受限的滚动容器（应用内容面板、Dialog 正文）在内容增长时自身盒子不变，
 * ResizeObserver 不会触发，必须靠这里重建滑块。
 */
const HOST_CONTENT_MUTATIONS: MutationObserverInit = {
  subtree: true,
  childList: true,
  characterData: true,
  attributes: true,
  attributeFilter: ["style", "class"],
};

/**
 * 安装全局浮层滚动条。
 *
 * 全局隐藏会占宽度的经典滚动槽，本模块只为显式注册的真实滚动宿主绘制独立滑块；
 * 动态列表和 Dialog 通过 v-overlay-scrollbar 在挂载时注册宿主。
 */
export function installOverlayScrollbars(): void {
  if (
    typeof window === "undefined" ||
    typeof document === "undefined" ||
    document.getElementById(ROOT_ID)
  ) {
    return;
  }

  overlayRoot = document.createElement("div");
  overlayRoot.id = ROOT_ID;
  overlayRoot.className = "app-overlay-scrollbars";
  overlayRoot.setAttribute("aria-hidden", "true");
  document.body.append(overlayRoot);

  activeMediaQuery = window.matchMedia(ACTIVE_MEDIA_QUERY);
  activeMediaQuery.addEventListener("change", scheduleRefresh);
  document.addEventListener("scroll", schedulePositionUpdate, true);
  document.addEventListener("transitionend", handleNavigationTransitionEnd, true);
  // 图片/字体加载会改变布局高度但不产生 DOM 变更，捕获 load 事件兜底重建滑块。
  window.addEventListener("load", scheduleRefresh, true);
  window.addEventListener("resize", scheduleRefresh);
  window.visualViewport?.addEventListener("resize", scheduleRefresh);
  window.visualViewport?.addEventListener("scroll", schedulePositionUpdate);

  if (typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(scheduleRefresh);
    for (const element of registeredElements) resizeObserver.observe(element);
  }

  for (const element of registeredElements) observeHostContent(element);

  scheduleRefresh();
}

/** 注册需要自绘滚动条的明确宿主，避免 resize 时扫描整个页面 DOM。 */
export function registerOverlayScrollbar(element: HTMLElement): void {
  registeredElements.add(element);
  resizeObserver?.observe(element);
  observeHostContent(element);
  scheduleRefresh();
}

/** 动态 Dialog 或页面卸载时移除滚动宿主及其滑块。 */
export function unregisterOverlayScrollbar(element: HTMLElement): void {
  registeredElements.delete(element);
  resizeObserver?.unobserve(element);
  contentObservers.get(element)?.disconnect();
  contentObservers.delete(element);
  const entry = entries.get(element);
  if (entry) removeEntry(entry);
}

/** 为宿主建立内容变更观察，检测影响滚动性的布局变化。 */
function observeHostContent(element: HTMLElement): void {
  if (typeof MutationObserver === "undefined" || contentObservers.has(element)) return;
  const observer = new MutationObserver(() => scheduleRefresh());
  observer.observe(element, HOST_CONTENT_MUTATIONS);
  contentObservers.set(element, observer);
}

function scheduleRefresh(): void {
  if (refreshFrame !== 0) return;
  refreshFrame = window.requestAnimationFrame(refreshEntries);
}

function schedulePositionUpdate(): void {
  if (refreshFrame !== 0) return;
  refreshFrame = window.requestAnimationFrame(updateEntryPositions);
}

/** 侧栏打开时首次检测发生在屏外；滑入结束后重新登记，恢复其浮层滚动条。 */
function handleNavigationTransitionEnd(event: TransitionEvent): void {
  if (
    event.propertyName === "transform" &&
    event.target instanceof HTMLElement &&
    event.target.matches(".app-navigation-pane")
  ) {
    scheduleRefresh();
  }
}

function refreshEntries(): void {
  refreshFrame = 0;
  if (!activeMediaQuery?.matches || !overlayRoot) {
    clearEntries();
    return;
  }

  const activeElements = new Set<HTMLElement>();

  for (const element of registeredElements) {
    if (!element.isConnected || overlayRoot.contains(element)) continue;
    const axes = getScrollableAxes(element);
    if (!axes.horizontal && !axes.vertical) continue;

    activeElements.add(element);
    let entry = entries.get(element);
    if (!entry) {
      entry = { element };
      entries.set(element, entry);
      resizeObserver?.observe(element);
    }
    syncEntryThumbs(entry, axes);
  }

  // 页面级滚动同样需要浮层滑块；文档滚动器不参与指令注册，单独按需纳入。
  const scrollingElement = document.scrollingElement;
  if (scrollingElement instanceof HTMLElement) {
    const axes = getScrollableAxes(scrollingElement);
    if (axes.horizontal || axes.vertical) {
      activeElements.add(scrollingElement);
      let entry = entries.get(scrollingElement);
      if (!entry) {
        entry = { element: scrollingElement };
        entries.set(scrollingElement, entry);
        resizeObserver?.observe(scrollingElement);
      }
      syncEntryThumbs(entry, axes);
    }
  }

  for (const [element, entry] of entries) {
    if (!activeElements.has(element)) removeEntry(entry);
  }

  updateEntryPositions();
}

function getScrollableAxes(element: HTMLElement): ScrollableAxes {
  if (!element.isConnected || element.clientWidth <= 0 || element.clientHeight <= 0) {
    return { horizontal: false, vertical: false };
  }

  const rect = element.getBoundingClientRect();
  if (rect.width <= 1 || rect.height <= 1) {
    return { horizontal: false, vertical: false };
  }

  const style = window.getComputedStyle(element);
  if (style.display === "none" || style.visibility === "hidden") {
    return { horizontal: false, vertical: false };
  }

  // 侧栏关闭时仍会短暂保留在 transform 退场动画中；其滑块应立即消失，避免停在旧位置。
  if (element.matches(".app-navigation-pane:not(.app-navigation-pane--open)")) {
    return { horizontal: false, vertical: false };
  }

  // 文档滚动器由浏览器按传播规则决定滚动性，无需依赖 overflow 声明。
  const isDocumentScroller = element === document.scrollingElement;

  return {
    horizontal:
      element.scrollWidth > element.clientWidth + 1 &&
      (isDocumentScroller || SCROLLABLE_OVERFLOW.has(style.overflowX)),
    vertical:
      element.scrollHeight > element.clientHeight + 1 &&
      (isDocumentScroller || SCROLLABLE_OVERFLOW.has(style.overflowY)),
  };
}

function syncEntryThumbs(entry: OverlayScrollbarEntry, axes: ScrollableAxes): void {
  if (axes.vertical && !entry.verticalThumb) {
    entry.verticalThumb = createThumb(entry, "vertical");
  } else if (!axes.vertical && entry.verticalThumb) {
    entry.verticalThumb.remove();
    entry.verticalThumb = undefined;
    entry.verticalGeometry = undefined;
  }

  if (axes.horizontal && !entry.horizontalThumb) {
    entry.horizontalThumb = createThumb(entry, "horizontal");
  } else if (!axes.horizontal && entry.horizontalThumb) {
    entry.horizontalThumb.remove();
    entry.horizontalThumb = undefined;
    entry.horizontalGeometry = undefined;
  }
}

function createThumb(entry: OverlayScrollbarEntry, axis: ScrollAxis): HTMLDivElement {
  const thumb = document.createElement("div");
  thumb.className = `app-overlay-scrollbar-thumb app-overlay-scrollbar-thumb--${axis}`;
  if (entry.element.matches(".app-navigation-pane")) {
    thumb.classList.add("app-overlay-scrollbar-thumb--navigation");
  }
  thumb.addEventListener("pointerdown", (event) => startThumbDrag(event, entry, axis));
  overlayRoot?.append(thumb);
  return thumb;
}

function updateEntryPositions(): void {
  refreshFrame = 0;
  if (!activeMediaQuery?.matches) return;

  // modal-layer 为全屏遮罩；只保留最顶层 Dialog 内的滑块，避免下层列表滑块穿透。
  const topModal = findTopmostModalLayer();

  for (const entry of entries.values()) {
    if (isObscuredByModal(entry.element, topModal)) {
      hideThumb(entry.verticalThumb);
      hideThumb(entry.horizontalThumb);
      continue;
    }

    const rect = getVisibleRect(entry.element);
    if (!rect || rect.width < MIN_THUMB_LENGTH || rect.height < MIN_THUMB_LENGTH) {
      hideThumb(entry.verticalThumb);
      hideThumb(entry.horizontalThumb);
      continue;
    }

    const zIndex = getOverlayZIndex(entry.element);
    positionVerticalThumb(entry, rect, zIndex);
    positionHorizontalThumb(entry, rect, zIndex);
  }
}

function positionVerticalThumb(
  entry: OverlayScrollbarEntry,
  rect: VisibleRect,
  zIndex: number,
): void {
  const thumb = entry.verticalThumb;
  if (!thumb) return;

  const trackLength = rect.height - TRACK_INSET * 2 - (entry.horizontalThumb ? THUMB_HIT_SIZE : 0);
  const maxScroll = entry.element.scrollHeight - entry.element.clientHeight;
  if (trackLength <= 0 || maxScroll <= 0) {
    hideThumb(thumb);
    return;
  }

  const thumbLength = Math.min(
    trackLength,
    Math.max(
      MIN_THUMB_LENGTH,
      trackLength * (entry.element.clientHeight / entry.element.scrollHeight),
    ),
  );
  const travel = Math.max(0, trackLength - thumbLength);
  const progress = Math.min(1, Math.max(0, entry.element.scrollTop / maxScroll));

  thumb.style.display = "block";
  thumb.style.zIndex = String(zIndex);
  thumb.style.left = `${rect.right - THUMB_HIT_SIZE}px`;
  thumb.style.top = `${rect.top + TRACK_INSET + travel * progress}px`;
  thumb.style.width = `${THUMB_HIT_SIZE}px`;
  thumb.style.height = `${thumbLength}px`;
  entry.verticalGeometry = { maxScroll, travel };
}

function positionHorizontalThumb(
  entry: OverlayScrollbarEntry,
  rect: VisibleRect,
  zIndex: number,
): void {
  const thumb = entry.horizontalThumb;
  if (!thumb) return;

  const trackLength = rect.width - TRACK_INSET * 2 - (entry.verticalThumb ? THUMB_HIT_SIZE : 0);
  const maxScroll = entry.element.scrollWidth - entry.element.clientWidth;
  if (trackLength <= 0 || maxScroll <= 0) {
    hideThumb(thumb);
    return;
  }

  const thumbLength = Math.min(
    trackLength,
    Math.max(
      MIN_THUMB_LENGTH,
      trackLength * (entry.element.clientWidth / entry.element.scrollWidth),
    ),
  );
  const travel = Math.max(0, trackLength - thumbLength);
  const progress = Math.min(1, Math.max(0, entry.element.scrollLeft / maxScroll));

  thumb.style.display = "block";
  thumb.style.zIndex = String(zIndex);
  thumb.style.left = `${rect.left + TRACK_INSET + travel * progress}px`;
  thumb.style.top = `${rect.bottom - THUMB_HIT_SIZE}px`;
  thumb.style.width = `${thumbLength}px`;
  thumb.style.height = `${THUMB_HIT_SIZE}px`;
  entry.horizontalGeometry = { maxScroll, travel };
}

function startThumbDrag(event: PointerEvent, entry: OverlayScrollbarEntry, axis: ScrollAxis): void {
  const thumb = axis === "vertical" ? entry.verticalThumb : entry.horizontalThumb;
  const geometry = axis === "vertical" ? entry.verticalGeometry : entry.horizontalGeometry;
  if (!thumb || !geometry || geometry.travel <= 0) return;

  event.preventDefault();
  event.stopPropagation();
  thumb.setPointerCapture(event.pointerId);
  thumb.classList.add("app-overlay-scrollbar-thumb--active");

  const startPointer = axis === "vertical" ? event.clientY : event.clientX;
  const startScroll = axis === "vertical" ? entry.element.scrollTop : entry.element.scrollLeft;

  const handleMove = (moveEvent: PointerEvent) => {
    const pointer = axis === "vertical" ? moveEvent.clientY : moveEvent.clientX;
    const nextScroll =
      startScroll + ((pointer - startPointer) / geometry.travel) * geometry.maxScroll;
    if (axis === "vertical") entry.element.scrollTop = nextScroll;
    else entry.element.scrollLeft = nextScroll;
    schedulePositionUpdate();
  };

  const finish = () => {
    thumb.classList.remove("app-overlay-scrollbar-thumb--active");
    thumb.removeEventListener("pointermove", handleMove);
    thumb.removeEventListener("pointerup", finish);
    thumb.removeEventListener("pointercancel", finish);
  };

  thumb.addEventListener("pointermove", handleMove);
  thumb.addEventListener("pointerup", finish);
  thumb.addEventListener("pointercancel", finish);
}

function getVisibleRect(element: HTMLElement): VisibleRect | null {
  const initialRect = element.getBoundingClientRect();
  let left = Math.max(0, initialRect.left);
  let top = Math.max(0, initialRect.top);
  let right = Math.min(window.innerWidth, initialRect.right);
  let bottom = Math.min(window.innerHeight, initialRect.bottom);

  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const style = window.getComputedStyle(parent);
    const parentRect = parent.getBoundingClientRect();
    if (style.overflowX !== "visible") {
      left = Math.max(left, parentRect.left);
      right = Math.min(right, parentRect.right);
    }
    if (style.overflowY !== "visible") {
      top = Math.max(top, parentRect.top);
      bottom = Math.min(bottom, parentRect.bottom);
    }
  }

  if (right <= left || bottom <= top) return null;
  return { bottom, height: bottom - top, left, right, top, width: right - left };
}

/** 读取宿主及其定位祖先中的最高 z-index；无定位上下文时为 0。 */
function getHostStackingZIndex(element: HTMLElement): number {
  let highestZIndex = 0;
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    const zIndex = Number.parseInt(window.getComputedStyle(current).zIndex, 10);
    if (Number.isFinite(zIndex)) highestZIndex = Math.max(highestZIndex, zIndex);
  }
  return highestZIndex;
}

/**
 * 滑块根层已在 --z-dialog-popover；此处只在同层内区分宿主优先级
 * （例如侧栏 / Dialog 内列表），默认略高于普通文档流。
 */
function getOverlayZIndex(element: HTMLElement): number {
  return Math.min(99, Math.max(18, getHostStackingZIndex(element)) + 1);
}

/** 按 z-index 与 DOM 顺序取当前最顶层的全屏 Dialog 遮罩。 */
function findTopmostModalLayer(): HTMLElement | null {
  let top: HTMLElement | null = null;
  let topZ = Number.NEGATIVE_INFINITY;
  for (const modal of document.querySelectorAll<HTMLElement>(".modal-layer")) {
    const zIndex = Number.parseInt(window.getComputedStyle(modal).zIndex, 10);
    const resolved = Number.isFinite(zIndex) ? zIndex : 0;
    // querySelectorAll 为树序；同等 z-index 时后挂载的 Dialog 胜出。
    if (resolved >= topZ) {
      top = modal;
      topZ = resolved;
    }
  }
  return top;
}

/**
 * 判断滚动宿主是否被顶层 Dialog 遮挡。
 *
 * - 顶层 modal-layer 内部的列表：保留滑块
 * - 页面或下层 Dialog 内容：隐藏滑块（根层 z-index 高于 Dialog，不隐藏会穿透）
 * - Teleport 到 body 且 z-index 高于顶层 Dialog 的浮层（如下拉选项）：保留滑块
 */
function isObscuredByModal(element: HTMLElement, topModal: HTMLElement | null): boolean {
  if (!topModal) return false;
  if (topModal.contains(element)) return false;

  const modalZ = Number.parseInt(window.getComputedStyle(topModal).zIndex, 10);
  const resolvedModalZ = Number.isFinite(modalZ) ? modalZ : 0;
  return getHostStackingZIndex(element) <= resolvedModalZ;
}

function hideThumb(thumb?: HTMLDivElement): void {
  if (thumb) thumb.style.display = "none";
}

function removeEntry(entry: OverlayScrollbarEntry): void {
  entry.verticalThumb?.remove();
  entry.horizontalThumb?.remove();
  entries.delete(entry.element);
}

function clearEntries(): void {
  for (const entry of [...entries.values()]) removeEntry(entry);
}
