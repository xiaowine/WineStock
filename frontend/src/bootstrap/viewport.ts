// 本文件拥有 Vue 挂载后的首帧视口稳定流程，属于 frontend 启动层；它不选择 Shell、不拥有平台生命周期。

const MOBILE_BREAKPOINT = 768;
const MIN_VALID_WIDTH = 320;
const VIEWPORT_META_SELECTOR = 'meta[name="viewport"]';

/**
 * 等待首帧布局完成，并在业务路由挂载前纠正移动 WebView 的临时宽布局视口。
 *
 * Vue mounted 只保证组件 DOM 已创建；这里额外等待多帧，确保 visualViewport、布局
 * 视口和 viewport meta 的变化完成后，业务页面才开始挂载和读取尺寸。
 */
export async function waitForStableViewport(): Promise<void> {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return;
  }

  // 第一帧让 Vue 完成 DOM 提交，第二帧让浏览器完成一次真实布局。
  await nextAnimationFrame();
  await nextAnimationFrame();

  // 修改 viewport meta 后再留出两帧，让媒体查询和 clientWidth 重新计算。
  normalizeViewportMeta();
  await nextAnimationFrame();
  await nextAnimationFrame();

  // 某些 WebView 的 visualViewport 在首个 resize 后才可读，再做有限次数的兜底测量。
  normalizeViewportMeta();
}

/**
 * 纠正部分移动 WebView 首帧暂时使用 980px 布局视口的问题。
 * 返回值表示本次是否修改了 viewport meta；正常桌面和正常移动视口不会修改它。
 */
export function normalizeViewportMeta(): boolean {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return false;
  }

  const meta = document.querySelector<HTMLMetaElement>(VIEWPORT_META_SELECTOR);
  const layoutWidth = window.innerWidth;
  const effectiveWidth = getEffectiveViewportWidth();
  if (!meta || layoutWidth <= 0 || effectiveWidth === null) {
    return false;
  }

  if (layoutWidth < MOBILE_BREAKPOINT || effectiveWidth >= MOBILE_BREAKPOINT) {
    return false;
  }

  const normalizedWidth = Math.max(MIN_VALID_WIDTH, Math.round(effectiveWidth));
  const content = `width=${normalizedWidth}, initial-scale=1.0, viewport-fit=cover`;
  if (meta.getAttribute("content") === content) {
    return false;
  }

  meta.setAttribute("content", content);
  return true;
}

function getEffectiveViewportWidth(): number | null {
  const widths = [
    window.innerWidth,
    window.visualViewport?.width,
    document.documentElement.clientWidth,
  ].filter((width): width is number => typeof width === "number" && width > 0);

  // 某些嵌入式 WebView 的 outerWidth 会先反映真实设备宽度，作为移动首帧的兜底。
  if (window.outerWidth >= MIN_VALID_WIDTH && window.outerWidth < MOBILE_BREAKPOINT) {
    widths.push(window.outerWidth);
  }

  return widths.length > 0 ? Math.min(...widths) : null;
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) {
        return;
      }
      settled = true;
      window.clearTimeout(fallbackTimer);
      resolve();
    };
    const fallbackTimer = window.setTimeout(finish, 100);
    window.requestAnimationFrame(finish);
  });
}
