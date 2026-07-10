// 本文件拥有 frontend refresh 的跨标签页互斥协调；它不保存 token 或执行鉴权网络请求。

const AUTH_REFRESH_LOCK_NAME = 'winestock.auth.refresh.v1'

/**
 * 在支持 Web Locks 的同源上下文间串行执行 refresh；不支持时由调用方重新读取 token 并单次重试。
 */
export function runWithAuthRefreshLock<TResult>(task: () => Promise<TResult>): Promise<TResult> {
  if (typeof navigator === 'undefined' || !navigator.locks) {
    return task()
  }

  return navigator.locks.request(
    AUTH_REFRESH_LOCK_NAME,
    { mode: 'exclusive' },
    async () => task(),
  )
}
