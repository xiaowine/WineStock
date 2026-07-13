// 本文件拥有前端 Shell 断点选择逻辑，属于 frontend；它不判断 Desktop、Android 或 Web 平台能力。
import { onBeforeUnmount, onMounted, ref } from 'vue'

const DESKTOP_QUERY = '(min-width: 768px)'
const DESKTOP_BREAKPOINT = 768

/** WebView 冷启动时布局视口可能暂为 980px，宿主窗口和可视视口会更早反映真实宽度。 */
function currentViewportWidth(): number {
  const widths = [
    window.innerWidth,
    window.visualViewport?.width,
    document.documentElement.clientWidth,
  ].filter((width): width is number => typeof width === 'number' && width > 0)
  // DevTools 或部分嵌入式宿主可能短暂报告小于可用移动视口的 outerWidth，不能让异常值覆盖布局视口。
  if (window.outerWidth >= 320) widths.push(window.outerWidth)
  return Math.min(...widths)
}

/** 按当前宿主与布局视口判断是否应使用桌面 Shell，可供交互分支在操作时即时核对。 */
export function isDesktopViewport(mediaQueryMatches?: boolean): boolean {
  if (typeof window === 'undefined') return true
  const queryMatches = mediaQueryMatches ?? window.matchMedia(DESKTOP_QUERY).matches
  return queryMatches && currentViewportWidth() >= DESKTOP_BREAKPOINT
}

/// 根据视口断点选择当前应挂载的应用壳，避免桌面和移动 Shell 同时渲染。
export function useResponsiveShell() {
  const isDesktop = ref(isDesktopViewport())
  let mediaQuery: MediaQueryList | undefined

  const updateShell = (query: MediaQueryList | MediaQueryListEvent) => {
    isDesktop.value = isDesktopViewport(query.matches)
  }

  const updateShellFromViewport = () => {
    if (mediaQuery) updateShell(mediaQuery)
  }

  onMounted(() => {
    mediaQuery = window.matchMedia(DESKTOP_QUERY)
    mediaQuery.addEventListener('change', updateShell)
    window.addEventListener('resize', updateShellFromViewport)
    window.visualViewport?.addEventListener('resize', updateShellFromViewport)
    updateShell(mediaQuery)
  })

  onBeforeUnmount(() => {
    window.removeEventListener('resize', updateShellFromViewport)
    window.visualViewport?.removeEventListener('resize', updateShellFromViewport)
    mediaQuery?.removeEventListener('change', updateShell)
  })

  return { isDesktop }
}
