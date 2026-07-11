// 本文件拥有 frontend 异步等待提示的延迟显示和最短展示调度；它不执行具体请求或决定提示内容。
import { onScopeDispose, readonly, ref, watch, type Ref } from 'vue'

/** 稳定等待提示的时间参数。 */
export interface StablePendingIndicatorOptions {
  /** 请求开始后经过多久才显示等待提示，避免快速操作闪烁。 */
  showDelayMs: number
  /** 等待提示一旦出现至少保持多久，避免出现后立即消失。 */
  minimumVisibleMs: number
}

/**
 * 把即时 pending 状态转换为视觉稳定的等待提示状态。
 * 快速完成时不显示提示；提示已经出现时会等待最短展示时间后再隐藏。
 */
export function useStablePendingIndicator(
  pending: Readonly<Ref<boolean>>,
  options: StablePendingIndicatorOptions,
): Readonly<Ref<boolean>> {
  const visible = ref(false)
  let visibleSince = 0
  let showTimer: ReturnType<typeof setTimeout> | null = null
  let hideTimer: ReturnType<typeof setTimeout> | null = null

  const stopWatching = watch(
    pending,
    (isPending) => {
      if (isPending) {
        clearTimer('hide')
        if (visible.value || showTimer) {
          return
        }

        showTimer = setTimeout(() => {
          showTimer = null
          if (!pending.value) {
            return
          }
          visibleSince = Date.now()
          visible.value = true
        }, options.showDelayMs)
        return
      }

      clearTimer('show')
      if (!visible.value) {
        return
      }

      const remaining = options.minimumVisibleMs - (Date.now() - visibleSince)
      if (remaining <= 0) {
        visible.value = false
        return
      }

      hideTimer = setTimeout(() => {
        hideTimer = null
        visible.value = false
      }, remaining)
    },
    { immediate: true, flush: 'sync' },
  )

  onScopeDispose(() => {
    stopWatching()
    clearTimer('show')
    clearTimer('hide')
  })

  function clearTimer(kind: 'show' | 'hide'): void {
    const timer = kind === 'show' ? showTimer : hideTimer
    if (timer === null) {
      return
    }
    clearTimeout(timer)
    if (kind === 'show') {
      showTimer = null
    } else {
      hideTimer = null
    }
  }

  return readonly(visible)
}
