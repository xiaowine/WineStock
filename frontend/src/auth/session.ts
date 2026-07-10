// 本文件拥有 frontend 鉴权会话、refresh 轮换和启动恢复；它不决定页面导航或平台生命周期。
import { readonly, shallowRef } from 'vue'
import { refresh, type AuthTokenResponse, type AuthUserResponse } from '../api/auth'
import { ApiError } from '../api/errors'
import { runWithAuthRefreshLock } from './coordination'
import {
  clearPersistedRefreshToken,
  clearPersistedRefreshTokenIfMatches,
  loadPersistedRefreshToken,
  persistRefreshToken,
  subscribePersistedRefreshTokenRemoval,
} from './storage'

const ACCESS_TOKEN_REFRESH_SKEW_MS = 30_000

/** 当前前端鉴权会话。 */
export interface AuthSession {
  /** JWT access token。 */
  accessToken: string
  /** access token 的本地预计过期时间，Unix 毫秒。 */
  accessTokenExpiresAt: number
  /** 登录用户摘要。 */
  user: AuthUserResponse
}

const mutableAuthSession = shallowRef<AuthSession | null>(null)
let refreshInFlight: Promise<AuthSession | null> | null = null
let stopPersistedSessionSynchronization: (() => void) | null = null

/** 只读当前会话；页面不得绕过会话函数直接修改 token。 */
export const authSession = readonly(mutableAuthSession)

/** 使用登录响应建立会话，并先持久化可供下次启动使用的 refresh token。 */
export function establishAuthSession(response: AuthTokenResponse): void {
  persistRefreshToken(response.refresh_token)
  mutableAuthSession.value = toAuthSession(response)
}

/** 清除当前内存会话和持久化 refresh token。 */
export function clearAuthSession(): void {
  mutableAuthSession.value = null
  clearPersistedRefreshToken()
}

/** 启动同源标签页间的持久会话清理同步；重复调用不会注册多个监听器。 */
export function startAuthSessionSynchronization(): void {
  if (stopPersistedSessionSynchronization) {
    return
  }

  stopPersistedSessionSynchronization = subscribePersistedRefreshTokenRemoval(() => {
    mutableAuthSession.value = null
  })
}

/** 应用启动时使用持久化 refresh token 恢复登录；无持久会话时返回 false。 */
export async function restoreAuthSession(): Promise<boolean> {
  return (await refreshAuthSession()) !== null
}

/** 向 API client 提供有效 access token；临近过期或强制刷新时执行 token 轮换。 */
export async function getValidAccessToken(forceRefresh = false): Promise<string | null> {
  const current = mutableAuthSession.value
  if (
    !forceRefresh &&
    current &&
    current.accessTokenExpiresAt > Date.now() + ACCESS_TOKEN_REFRESH_SKEW_MS
  ) {
    return current.accessToken
  }

  return (await refreshAuthSession())?.accessToken ?? null
}

/** 串行执行 refresh token 轮换，避免并发请求重复使用已经失效的旧 token。 */
export async function refreshAuthSession(): Promise<AuthSession | null> {
  if (refreshInFlight) {
    return refreshInFlight
  }

  refreshInFlight = performRefresh()
  try {
    return await refreshInFlight
  } finally {
    refreshInFlight = null
  }
}

async function performRefresh(): Promise<AuthSession | null> {
  return runWithAuthRefreshLock(performRefreshWithLatestPersistedToken)
}

/**
 * 获得跨标签页锁后读取最新 refresh token；无锁环境遇到旧 token 时最多改用新记录重试一次。
 */
async function performRefreshWithLatestPersistedToken(): Promise<AuthSession | null> {
  let attemptedRefreshToken = loadPersistedRefreshToken()

  for (let attempt = 0; attempt < 2; attempt += 1) {
    if (!attemptedRefreshToken) {
      return null
    }

    try {
      const response = await refresh({ refresh_token: attemptedRefreshToken })
      persistRefreshToken(response.refresh_token)
      const session = toAuthSession(response)
      mutableAuthSession.value = session
      return session
    } catch (error) {
      if (!(error instanceof ApiError) || error.code !== 'invalid_refresh_token') {
        throw error
      }

      const latestRefreshToken = loadPersistedRefreshToken()
      if (
        attempt === 0 &&
        latestRefreshToken &&
        latestRefreshToken !== attemptedRefreshToken
      ) {
        attemptedRefreshToken = latestRefreshToken
        continue
      }

      mutableAuthSession.value = null
      if (latestRefreshToken === attemptedRefreshToken) {
        clearPersistedRefreshTokenIfMatches(attemptedRefreshToken)
      }
      return null
    }
  }

  return null
}

function toAuthSession(response: AuthTokenResponse): AuthSession {
  return {
    accessToken: response.access_token,
    accessTokenExpiresAt: Date.now() + response.expires_in * 1000,
    user: response.user,
  }
}
