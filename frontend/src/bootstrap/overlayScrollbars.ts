// 本文件拥有移动与触控视口的全局浮层滚动条，属于 frontend 启动层；它不改变业务滚动容器的所有权。

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

const ACTIVE_MEDIA_QUERY = "(max-width: 767px), ((hover: none) and (pointer: coarse))";
const ROOT_ID = "app-overlay-scrollbars";
const TRACK_INSET = 3;
const THUMB_HIT_SIZE = 12;
const MIN_THUMB_LENGTH = 32;
const SCROLLABLE_OVERFLOW = new Set(["auto", "scroll"]);

let activeMediaQuery: MediaQueryList | undefined;
let mutationObserver: MutationObserver | undefined;
let overlayRoot: HTMLDivElement | undefined;
let refreshFrame = 0;
let resizeObserver: ResizeObserver | undefined;
const entries = new Map<HTMLElement, OverlayScrollbarEntry>();

/**
 * 安装全局浮层滚动条。
 *
 * 移动布局隐藏会占宽度的经典滚动槽，本模块为当前真实滚动宿主绘制独立滑块；
 * 响应式切换滚动容器、Dialog Teleport 和动态列表变化都会重新核对宿主。
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
  window.addEventListener("resize", scheduleRefresh);
  window.visualViewport?.addEventListener("resize", scheduleRefresh);
  window.visualViewport?.addEventListener("scroll", schedulePositionUpdate);

  mutationObserver = new MutationObserver((records) => {
    if (overlayRoot && records.every((record) => overlayRoot?.contains(record.target))) {
      return;
    }
    scheduleRefresh();
  });
  mutationObserver.observe(document.body, {
    attributes: true,
    attributeFilter: ["class", "hidden", "style"],
    childList: true,
    subtree: true,
  });

  if (typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(scheduleRefresh);
  }

  scheduleRefresh();
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
  const scrollingElement = document.scrollingElement;
  const candidates = Array.from(document.querySelectorAll<HTMLElement>("body *"));
  if (scrollingElement instanceof HTMLElement) candidates.unshift(scrollingElement);

  for (const element of candidates) {
    if (overlayRoot.contains(element)) continue;
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

  for (const entry of entries.values()) {
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

function getOverlayZIndex(element: HTMLElement): number {
  let highestZIndex = 18;
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    const zIndex = Number.parseInt(window.getComputedStyle(current).zIndex, 10);
    if (Number.isFinite(zIndex)) highestZIndex = Math.max(highestZIndex, zIndex);
  }
  return Math.min(99, highestZIndex + 1);
}

function hideThumb(thumb?: HTMLDivElement): void {
  if (thumb) thumb.style.display = "none";
}

function removeEntry(entry: OverlayScrollbarEntry): void {
  resizeObserver?.unobserve(entry.element);
  entry.verticalThumb?.remove();
  entry.horizontalThumb?.remove();
  entries.delete(entry.element);
}

function clearEntries(): void {
  for (const entry of [...entries.values()]) removeEntry(entry);
}
