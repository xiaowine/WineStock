// 本文件拥有 frontend access token 的定时刷新与页面唤醒补检；它不保存 token、实现 HTTP 请求或决定路由导航。
import { watch } from 'vue'
import {
  authSession,
  authStatus,
  ensureAuthSessionInitialized,
  isLoggingOut,
  refreshAuthSession,
} from './session'

const AUTO_REFRESH_LEAD_MS = 60_000
const AUTO_REFRESH_JITTER_MS = 10_000
const AUTO_REFRESH_RETRY_MS = 30_000
const AUTO_REFRESH_MIN_DELAY_MS = 1_000

let autoRefreshStarted = false
let autoRefreshTimer: number | null = null
let autoRefreshInFlight: Promise<void> | null = null
let tabRefreshJitterMs = 0

/**
 * 启动一次 access token 自动刷新调度。
 * 定时器接近过期时主动轮换；浏览器节流导致错过定时器时，由焦点、可见性和联网事件补检。
 */
export function startAuthSessionAutoRefresh(): void {
  if (autoRefreshStarted || typeof window === 'undefined' || typeof document === 'undefined') {
    return
  }

  autoRefreshStarted = true
  tabRefreshJitterMs = Math.floor(Math.random() * AUTO_REFRESH_JITTER_MS)

  watch([authSession, authStatus, isLoggingOut], scheduleNextAutomaticRefresh, {
    flush: 'sync',
  })
  window.addEventListener('focus', handleSessionWake)
  window.addEventListener('online', handleSessionWake)
  document.addEventListener('visibilitychange', handleVisibilityChange)

  scheduleNextAutomaticRefresh()
}

function scheduleNextAutomaticRefresh(): void {
  clearAutomaticRefreshTimer()
  if (isLoggingOut.value) {
    return
  }

  if (authStatus.value === 'unavailable') {
    scheduleAutomaticRefresh(AUTO_REFRESH_RETRY_MS + tabRefreshJitterMs)
    return
  }

  const session = authSession.value
  if (authStatus.value !== 'authenticated' || !session) {
    return
  }

  const delay = Math.max(
    AUTO_REFRESH_MIN_DELAY_MS,
    session.accessTokenExpiresAt - Date.now() - AUTO_REFRESH_LEAD_MS + tabRefreshJitterMs,
  )
  scheduleAutomaticRefresh(delay)
}

function scheduleAutomaticRefresh(delayMs: number): void {
  autoRefreshTimer = window.setTimeout(() => {
    autoRefreshTimer = null
    void triggerAutomaticRefresh()
  }, delayMs)
}

function clearAutomaticRefreshTimer(): void {
  if (autoRefreshTimer === null) {
    return
  }
  window.clearTimeout(autoRefreshTimer)
  autoRefreshTimer = null
}

/** 接近过期时 refresh；unavailable 状态通过统一初始化入口重试并保留原有错误语义。 */
function triggerAutomaticRefresh(): Promise<void> {
  if (autoRefreshInFlight) {
    return autoRefreshInFlight
  }

  const task = performAutomaticRefresh()
    .catch(() => {
      // 会话层已经把网络、配置和响应错误转换为 unavailable，调度层只负责稍后重试。
    })
    .finally(() => {
      autoRefreshInFlight = null
      scheduleNextAutomaticRefresh()
    })
  autoRefreshInFlight = task
  return task
}

async function performAutomaticRefresh(): Promise<void> {
  if (isLoggingOut.value) {
    return
  }
  if (authStatus.value === 'unavailable') {
    await ensureAuthSessionInitialized()
    return
  }

  const session = authSession.value
  if (authStatus.value !== 'authenticated' || !session) {
    return
  }
  if (session.accessTokenExpiresAt > Date.now() + AUTO_REFRESH_LEAD_MS) {
    return
  }

  await refreshAuthSession()
}

function handleSessionWake(): void {
  scheduleNextAutomaticRefresh()

  const session = authSession.value
  const shouldRetryUnavailable = authStatus.value === 'unavailable'
  const shouldRefreshExpiringSession =
    authStatus.value === 'authenticated' &&
    session !== null &&
    session.accessTokenExpiresAt <= Date.now() + AUTO_REFRESH_LEAD_MS
  if (!isLoggingOut.value && (shouldRetryUnavailable || shouldRefreshExpiringSession)) {
    void triggerAutomaticRefresh()
  }
}

function handleVisibilityChange(): void {
  if (document.visibilityState === 'visible') {
    handleSessionWake()
  }
}
