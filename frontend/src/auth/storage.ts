// 本文件拥有 frontend refresh token 的统一 localStorage 持久化；它不保存 access token 或执行网络刷新。
import { resolveApiBaseUrl } from "../api/runtime-config";

const STORAGE_KEY = "winestock.auth.session.v1";
const STORAGE_VERSION = 1;

/** localStorage 中的版本化 refresh 会话记录。 */
interface PersistedRefreshSession {
  /** 存储结构版本，用于拒绝不兼容的旧记录。 */
  version: number;
  /** 获取该 refresh token 的服务根地址。 */
  api_base_url: string;
  /** 用于重新建立会话的 opaque refresh token。 */
  refresh_token: string;
  /** 最近保存时间，Unix 毫秒。 */
  saved_at: number;
}

/** 取消 refresh token 跨标签页移除监听的函数。 */
export type StopPersistedRefreshTokenSubscription = () => void;

/** 浏览器持久化不可用、读取失败或写入失败。 */
export class AuthPersistenceError extends Error {
  constructor(message: string, cause?: unknown) {
    super(message, { cause });
    this.name = "AuthPersistenceError";
  }
}

/** 读取当前 API 服务对应的 refresh token；损坏或不兼容记录会被清除。 */
export function loadPersistedRefreshToken(): string | null {
  const storage = resolveLocalStorage();
  let serialized: string | null;
  try {
    serialized = storage.getItem(STORAGE_KEY);
  } catch (error) {
    throw new AuthPersistenceError("无法读取本地登录状态", error);
  }

  if (!serialized) {
    return null;
  }

  const parsed = parsePersistedRefreshSession(serialized);
  if (!parsed) {
    removeInvalidRecord(storage);
    return null;
  }

  return parsed.api_base_url === resolveApiBaseUrl() ? parsed.refresh_token : null;
}

/** 保存当前 API 服务的 refresh token；新 token 会覆盖已经轮换失效的旧值。 */
export function persistRefreshToken(refreshToken: string): void {
  const record: PersistedRefreshSession = {
    version: STORAGE_VERSION,
    api_base_url: resolveApiBaseUrl(),
    refresh_token: refreshToken,
    saved_at: Date.now(),
  };

  try {
    resolveLocalStorage().setItem(STORAGE_KEY, JSON.stringify(record));
  } catch (error) {
    throw new AuthPersistenceError("无法保存登录状态", error);
  }
}

/** 清除统一持久化会话；登出和 refresh token 失效时调用。 */
export function clearPersistedRefreshToken(): void {
  try {
    resolveLocalStorage().removeItem(STORAGE_KEY);
  } catch (error) {
    throw new AuthPersistenceError("无法清除本地登录状态", error);
  }
}

/**
 * 仅当当前服务仍保存指定 refresh token 时清除记录。
 * 用于避免旧标签页的失败响应删除其它标签页已经轮换得到的新 token。
 */
export function clearPersistedRefreshTokenIfMatches(expectedRefreshToken: string): boolean {
  const storage = resolveLocalStorage();
  let serialized: string | null;
  try {
    serialized = storage.getItem(STORAGE_KEY);
  } catch (error) {
    throw new AuthPersistenceError("无法读取待清除的本地登录状态", error);
  }

  if (!serialized) {
    return false;
  }

  const parsed = parsePersistedRefreshSession(serialized);
  if (!parsed) {
    removeInvalidRecord(storage);
    return false;
  }
  if (
    parsed.api_base_url !== resolveApiBaseUrl() ||
    parsed.refresh_token !== expectedRefreshToken
  ) {
    return false;
  }

  try {
    if (storage.getItem(STORAGE_KEY) !== serialized) {
      return false;
    }
    storage.removeItem(STORAGE_KEY);
    return true;
  } catch (error) {
    throw new AuthPersistenceError("无法清除失效的本地登录状态", error);
  }
}

/**
 * 监听其它同源文档移除 refresh token；写入方不会收到自己的 storage 事件。
 * 当前仅同步清理动作，token 更新由每次 refresh 获得锁后重新读取 localStorage。
 */
export function subscribePersistedRefreshTokenRemoval(
  listener: () => void,
): StopPersistedRefreshTokenSubscription {
  if (typeof window === "undefined") {
    return () => undefined;
  }

  let storage: Storage;
  try {
    storage = resolveLocalStorage();
  } catch {
    return () => undefined;
  }

  const handleStorage = (event: StorageEvent): void => {
    const removesAuthRecord =
      event.storageArea === storage &&
      event.newValue === null &&
      (event.key === STORAGE_KEY || event.key === null);
    if (removesAuthRecord) {
      listener();
    }
  };

  window.addEventListener("storage", handleStorage);
  return () => window.removeEventListener("storage", handleStorage);
}

function resolveLocalStorage(): Storage {
  if (typeof window === "undefined" || !window.localStorage) {
    throw new AuthPersistenceError("当前运行环境不支持 localStorage");
  }
  return window.localStorage;
}

function isPersistedRefreshSession(value: unknown): value is PersistedRefreshSession {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const record = value as Record<string, unknown>;
  return (
    record.version === STORAGE_VERSION &&
    typeof record.api_base_url === "string" &&
    typeof record.refresh_token === "string" &&
    record.refresh_token.length > 0 &&
    record.refresh_token.length <= 512 &&
    typeof record.saved_at === "number" &&
    Number.isFinite(record.saved_at)
  );
}

function parsePersistedRefreshSession(serialized: string): PersistedRefreshSession | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(serialized) as unknown;
  } catch {
    return null;
  }

  return isPersistedRefreshSession(parsed) ? parsed : null;
}

function removeInvalidRecord(storage: Storage): void {
  try {
    storage.removeItem(STORAGE_KEY);
  } catch (error) {
    throw new AuthPersistenceError("无法清除损坏的本地登录状态", error);
  }
}
