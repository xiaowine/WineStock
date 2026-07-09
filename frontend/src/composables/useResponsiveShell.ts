// 本文件拥有前端 Shell 断点选择逻辑，属于 frontend；它不判断 Desktop、Android 或 Web 平台能力。
import { onBeforeUnmount, onMounted, ref } from 'vue'

const DESKTOP_QUERY = '(min-width: 768px)'

/// 根据视口断点选择当前应挂载的应用壳，避免桌面和移动 Shell 同时渲染。
export function useResponsiveShell() {
  const isDesktop = ref(typeof window === 'undefined' ? true : window.matchMedia(DESKTOP_QUERY).matches)
  let mediaQuery: MediaQueryList | undefined

  const updateShell = (query: MediaQueryList | MediaQueryListEvent) => {
    isDesktop.value = query.matches
  }

  onMounted(() => {
    mediaQuery = window.matchMedia(DESKTOP_QUERY)
    updateShell(mediaQuery)
    mediaQuery.addEventListener('change', updateShell)
  })

  onBeforeUnmount(() => {
    mediaQuery?.removeEventListener('change', updateShell)
  })

  return { isDesktop }
}
