// 本文件拥有 frontend 鉴权会话、启动恢复、refresh 轮换和登出用例；它不决定页面导航或平台生命周期。
import { readonly, ref, shallowRef } from "vue";
import { logout, refresh, type AuthTokenResponse, type AuthUserResponse } from "../api/auth";
import { ApiError } from "../api/errors";
import { runWithAuthSessionLock } from "./coordination";
import {
  clearPersistedRefreshToken,
  clearPersistedRefreshTokenIfMatches,
  loadPersistedRefreshToken,
  persistRefreshToken,
  subscribePersistedRefreshTokenRemoval,
} from "./storage";

const ACCESS_TOKEN_REFRESH_SKEW_MS = 30_000;

/** 会话初始化和可用性状态；只有 anonymous 表示已经明确未登录。 */
export type AuthStatus = "idle" | "restoring" | "authenticated" | "anonymous" | "unavailable";

/** 登出结果：已吊销、原 token 已失效，或仅完成本地退出。 */
export type LogoutResult = "revoked" | "already_invalid" | "local_only";

/** 当前前端鉴权会话。 */
export interface AuthSession {
  /** JWT access token。 */
  accessToken: string;
  /** access token 的本地预计过期时间，Unix 毫秒。 */
  accessTokenExpiresAt: number;
  /** 登录用户摘要。 */
  user: AuthUserResponse;
}

const mutableAuthSession = shallowRef<AuthSession | null>(null);
const mutableAuthStatus = ref<AuthStatus>("idle");
const mutableIsLoggingOut = ref(false);
let initializationInFlight: Promise<AuthStatus> | null = null;
let refreshInFlight: Promise<AuthSession | null> | null = null;
let logoutInFlight: Promise<LogoutResult> | null = null;
let stopPersistedSessionSynchronization: (() => void) | null = null;
let runtimeGeneration = 0;

/** 只读当前会话；页面不得绕过会话函数直接修改 token。 */
export const authSession = readonly(mutableAuthSession);

/** 只读会话状态；路由只在 anonymous 时判定用户明确未登录。 */
export const authStatus = readonly(mutableAuthStatus);

/** 只读登出进行状态，供所有平台入口防止重复提交。 */
export const isLoggingOut = readonly(mutableIsLoggingOut);

/** 使用登录响应建立会话，并先持久化可供下次启动使用的 refresh token。 */
export function establishAuthSession(response: AuthTokenResponse): void {
  persistRefreshToken(response.refresh_token);
  mutableAuthSession.value = toAuthSession(response);
  mutableAuthStatus.value = "authenticated";
}

/** 改密接口成功后清除当前会话的强制改密标记；token 和权限保持不变。 */
export function markPasswordChangeCompleted(): void {
  const current = mutableAuthSession.value;
  if (!current) {
    return;
  }

  mutableAuthSession.value = {
    ...current,
    user: {
      ...current.user,
      password_change_required: false,
    },
  };
}

/** 当前用户权限被管理接口修改后同步会话快照；后续请求仍由后端重新执行授权。 */
export function replaceCurrentSessionPermissions(
  userId: number,
  permissions: readonly string[],
): void {
  const current = mutableAuthSession.value;
  if (!current || current.user.id !== String(userId)) {
    return;
  }

  mutableAuthSession.value = {
    ...current,
    user: {
      ...current.user,
      permissions: [...permissions],
    },
  };
}

/** 启动同源标签页间的持久会话清理同步；重复调用不会注册多个监听器。 */
export function startAuthSessionSynchronization(): void {
  if (stopPersistedSessionSynchronization) {
    return;
  }

  stopPersistedSessionSynchronization = subscribePersistedRefreshTokenRemoval(() => {
    mutableAuthSession.value = null;
    mutableAuthStatus.value = "anonymous";
  });
}

/**
 * 确保启动会话只恢复一次；网络或配置失败返回 unavailable，并保留持久 refresh token。
 * 在 unavailable 状态再次调用会主动重试，使后续导航可以恢复连接。
 */
export function ensureAuthSessionInitialized(): Promise<AuthStatus> {
  if (mutableAuthStatus.value === "authenticated" || mutableAuthStatus.value === "anonymous") {
    return Promise.resolve(mutableAuthStatus.value);
  }
  if (initializationInFlight) {
    return initializationInFlight;
  }

  mutableAuthStatus.value = "restoring";
  const generation = runtimeGeneration;
  const task = performInitialization(generation).finally(() => {
    initializationInFlight = null;
  });
  initializationInFlight = task;
  return task;
}

/** 向 API client 提供有效 access token；临近过期或强制刷新时执行 token 轮换。 */
export async function getValidAccessToken(forceRefresh = false): Promise<string | null> {
  if (mutableIsLoggingOut.value) {
    return null;
  }

  const current = mutableAuthSession.value;
  if (
    !forceRefresh &&
    current &&
    current.accessTokenExpiresAt > Date.now() + ACCESS_TOKEN_REFRESH_SKEW_MS
  ) {
    return current.accessToken;
  }

  return (await refreshAuthSession())?.accessToken ?? null;
}

/** 串行执行 refresh token 轮换；非凭据错误会标记服务暂不可用，但不会删除持久 token。 */
export async function refreshAuthSession(): Promise<AuthSession | null> {
  if (mutableIsLoggingOut.value) {
    return null;
  }
  if (refreshInFlight) {
    return refreshInFlight;
  }

  const generation = runtimeGeneration;
  const task = performRefreshAndUpdateStatus(generation).finally(() => {
    refreshInFlight = null;
  });
  refreshInFlight = task;
  return task;
}

/**
 * 吊销跨标签页共享的最新 refresh token，并在任何服务端结果下清除本机会话。
 * 同标签页重复调用复用一个 Promise，避免发送多个登出请求。
 */
export function logoutAuthSession(): Promise<LogoutResult> {
  if (logoutInFlight) {
    return logoutInFlight;
  }

  mutableIsLoggingOut.value = true;
  const task = performLogout().finally(() => {
    mutableIsLoggingOut.value = false;
    logoutInFlight = null;
  });
  logoutInFlight = task;
  return task;
}

/**
 * API 根地址切换时清除仅属于旧服务的内存会话。
 * 持久 refresh token 继续保留其原 API 绑定，切换回旧服务时仍可按现有规则恢复。
 */
export function resetAuthSessionForRuntimeChange(): void {
  runtimeGeneration += 1;
  mutableAuthSession.value = null;
  mutableAuthStatus.value = "idle";
  initializationInFlight = null;
  refreshInFlight = null;
}

async function performInitialization(generation: number): Promise<AuthStatus> {
  try {
    await refreshAuthSession();
  } catch {
    if (generation === runtimeGeneration && !mutableIsLoggingOut.value) {
      mutableAuthStatus.value = "unavailable";
    }
  }
  return mutableAuthStatus.value;
}

async function performRefreshAndUpdateStatus(generation: number): Promise<AuthSession | null> {
  try {
    const session = await runWithAuthSessionLock(() =>
      performRefreshWithLatestPersistedToken(generation),
    );
    if (generation === runtimeGeneration && !mutableIsLoggingOut.value) {
      mutableAuthStatus.value = session ? "authenticated" : "anonymous";
    }
    return session;
  } catch (error) {
    if (generation === runtimeGeneration && !mutableIsLoggingOut.value) {
      mutableAuthStatus.value = "unavailable";
    }
    throw error;
  }
}

/**
 * 获得跨标签页锁后读取最新 refresh token；无锁环境遇到旧 token 时最多改用新记录重试一次。
 */
async function performRefreshWithLatestPersistedToken(
  generation: number,
): Promise<AuthSession | null> {
  let attemptedRefreshToken = loadPersistedRefreshToken();

  for (let attempt = 0; attempt < 2; attempt += 1) {
    if (!attemptedRefreshToken) {
      if (generation === runtimeGeneration) {
        mutableAuthSession.value = null;
      }
      return null;
    }

    try {
      const response = await refresh({ refresh_token: attemptedRefreshToken });
      if (generation !== runtimeGeneration) {
        return null;
      }
      persistRefreshToken(response.refresh_token);
      const session = toAuthSession(response);
      mutableAuthSession.value = session;
      return session;
    } catch (error) {
      if (!(error instanceof ApiError) || error.code !== "invalid_refresh_token") {
        throw error;
      }
      if (generation !== runtimeGeneration) {
        return null;
      }

      const latestRefreshToken = loadPersistedRefreshToken();
      if (attempt === 0 && latestRefreshToken && latestRefreshToken !== attemptedRefreshToken) {
        attemptedRefreshToken = latestRefreshToken;
        continue;
      }

      mutableAuthSession.value = null;
      if (latestRefreshToken === attemptedRefreshToken) {
        clearPersistedRefreshTokenIfMatches(attemptedRefreshToken);
      }
      return null;
    }
  }

  return null;
}

async function performLogout(): Promise<LogoutResult> {
  let result: LogoutResult = "local_only";
  try {
    const activeRefresh = refreshInFlight;
    if (activeRefresh) {
      try {
        await activeRefresh;
      } catch {
        // refresh 失败不阻止登出继续读取并吊销仍持久化的最新 token。
      }
    }
    result = await runWithAuthSessionLock(performLogoutWithLatestPersistedToken);
  } catch {
    result = "local_only";
  } finally {
    clearLocalAuthSession();
  }
  return result;
}

/** 锁内吊销最新 token；无锁环境遇到其它标签页已轮换的旧 token 时最多追赶一次。 */
async function performLogoutWithLatestPersistedToken(): Promise<LogoutResult> {
  let attemptedRefreshToken = loadPersistedRefreshToken();

  for (let attempt = 0; attempt < 2; attempt += 1) {
    if (!attemptedRefreshToken) {
      return "already_invalid";
    }

    try {
      await logout({ refresh_token: attemptedRefreshToken });
      return "revoked";
    } catch (error) {
      if (!(error instanceof ApiError) || error.code !== "invalid_refresh_token") {
        throw error;
      }

      const latestRefreshToken = loadPersistedRefreshToken();
      if (attempt === 0 && latestRefreshToken && latestRefreshToken !== attemptedRefreshToken) {
        attemptedRefreshToken = latestRefreshToken;
        continue;
      }
      return "already_invalid";
    }
  }

  return "already_invalid";
}

/** 当前标签页明确退出时先清空内存，再移除持久 token 以通知其它标签页。 */
function clearLocalAuthSession(): void {
  mutableAuthSession.value = null;
  mutableAuthStatus.value = "anonymous";
  clearPersistedRefreshToken();
}

function toAuthSession(response: AuthTokenResponse): AuthSession {
  return {
    accessToken: response.access_token,
    accessTokenExpiresAt: Date.now() + response.expires_in * 1000,
    user: response.user,
  };
}
