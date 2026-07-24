// 本文件实现普通浏览器和 Vite 开发环境的 Shell Bridge fallback；它不启动本地 Axum 或访问平台文件系统。
import { ApiConfigurationError } from "../api/errors";
import { normalizeApiBaseUrl, resolveInitialApiBaseUrl } from "../api/runtime-config";
import {
  cloneRuntimeConfig,
  cloneRuntimeSnapshot,
  defaultRuntimeConfig,
  SHELL_BRIDGE_PROTOCOL_VERSION,
  type EditableRuntimeConfig,
  type RuntimeConfigField,
  type RuntimeConfigValidationResult,
  type RuntimeSnapshot,
  type ShellBridge,
} from "./contract";

const STORAGE_KEY = "winestock.runtime.config.v1";
const listeners = new Set<(snapshot: RuntimeSnapshot) => void>();
const resumeListeners = new Set<() => void>();
let snapshot: RuntimeSnapshot | null = null;
let resumeListenerInstalled = false;

/** 创建不依赖 native API 的 Web fallback，供浏览器开发和静态部署使用。 */
export function createWebShellBridge(): ShellBridge {
  installResumeListener();
  return {
    async getRuntimeSnapshot() {
      snapshot ??= loadInitialSnapshot();
      return cloneRuntimeSnapshot(snapshot);
    },
    async validateRuntimeConfig(config) {
      return validateRuntimeConfig(config);
    },
    async applyRuntimeConfig(config) {
      snapshot ??= loadInitialSnapshot();
      const normalizedConfig = normalizeWebRuntimeConfig(config);
      const validation = validateRuntimeConfig(normalizedConfig);
      if (!validation.valid) {
        return {
          ...validation,
          applied: false,
          snapshot: cloneRuntimeSnapshot(snapshot),
        };
      }

      const nextSnapshot = createConfiguredSnapshot(normalizedConfig);
      try {
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(normalizedConfig));
      } catch (error) {
        return {
          valid: true,
          fieldErrors: {},
          applied: false,
          snapshot: cloneRuntimeSnapshot(snapshot),
          error: {
            code: "config_unavailable",
            message: "浏览器无法保存运行配置，请检查本地存储权限",
          },
        };
      }

      snapshot = nextSnapshot;
      publishSnapshot();
      return {
        valid: true,
        fieldErrors: {},
        applied: true,
        snapshot: cloneRuntimeSnapshot(nextSnapshot),
      };
    },
    async startLocalService() {
      return unsupportedLocalServiceOperation();
    },
    async stopLocalService() {
      return unsupportedLocalServiceOperation();
    },
    async restartLocalService() {
      return unsupportedLocalServiceOperation();
    },
    async frontendReady() {
      return undefined;
    },
    async openExternal(url) {
      const normalized = normalizeExternalUrl(url);
      window.open(normalized, "_blank", "noopener,noreferrer");
    },
    async onRuntimeStateChanged(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async onAppResumed(listener) {
      resumeListeners.add(listener);
      return () => resumeListeners.delete(listener);
    },
  };
}

function loadInitialSnapshot(): RuntimeSnapshot {
  const persisted = loadPersistedConfig();
  if (persisted.status === "loaded") {
    const validation = validateRuntimeConfig(persisted.config);
    if (validation.valid) {
      return createConfiguredSnapshot(persisted.config);
    }
    return {
      protocolVersion: SHELL_BRIDGE_PROTOCOL_VERSION,
      platform: "web",
      configStatus: "invalid",
      config: cloneRuntimeConfig(persisted.config),
      initialized: false,
      service: {
        ownership: isRemoteRuntimeMode(persisted.config.mode) ? "remote" : "local",
        phase: "stopped",
        error: {
          code: "config_invalid",
          message: "已保存的运行配置无效，请修正后重新应用",
        },
      },
      capabilities: createWebCapabilities(),
    };
  }

  if (persisted.status === "invalid") {
    return {
      protocolVersion: SHELL_BRIDGE_PROTOCOL_VERSION,
      platform: "web",
      configStatus: "invalid",
      config: cloneRuntimeConfig(defaultRuntimeConfig),
      initialized: false,
      service: {
        ownership: "local",
        phase: "stopped",
        error: {
          code: "config_invalid",
          message: "已保存的运行配置无法解析，请重新应用默认配置",
        },
      },
      capabilities: createWebCapabilities(),
    };
  }

  const initialApiBaseUrl = resolveInitialApiBaseUrl();
  if (initialApiBaseUrl) {
    return createConfiguredSnapshot({
      ...defaultRuntimeConfig,
      mode: "client-only",
      remoteBaseUrl: initialApiBaseUrl,
    });
  }

  return {
    protocolVersion: SHELL_BRIDGE_PROTOCOL_VERSION,
    platform: "web",
    configStatus: "unconfigured",
    config: cloneRuntimeConfig(defaultRuntimeConfig),
    initialized: false,
    service: {
      ownership: "local",
      phase: "stopped",
    },
    capabilities: createWebCapabilities(),
  };
}

function isRemoteRuntimeMode(mode: EditableRuntimeConfig["mode"]): boolean {
  return mode === "client-only" || mode === "connect-to-remote";
}

function createConfiguredSnapshot(config: EditableRuntimeConfig): RuntimeSnapshot {
  config = normalizeWebRuntimeConfig(config);
  const remote = isRemoteRuntimeMode(config.mode);
  const apiBaseUrl = remote
    ? normalizeApiBaseUrl(config.remoteBaseUrl)
    : `http://127.0.0.1:${config.port}`;
  return {
    protocolVersion: SHELL_BRIDGE_PROTOCOL_VERSION,
    platform: "web",
    configStatus: "configured",
    config: cloneRuntimeConfig(config),
    initialized: true,
    service: {
      ownership: remote ? "remote" : "local",
      phase: "running",
      apiBaseUrl,
      boundAddress: remote ? undefined : `${config.bindHost}:${config.port}`,
    },
    capabilities: createWebCapabilities(),
  };
}

function createWebCapabilities(): RuntimeSnapshot["capabilities"] {
  return {
    startLocalService: false,
    stopLocalService: false,
    restartLocalService: false,
    nativeBack: false,
    openExternal: true,
    serverMode: true,
  };
}

function validateRuntimeConfig(config: EditableRuntimeConfig): RuntimeConfigValidationResult {
  const fieldErrors: Partial<Record<RuntimeConfigField, string[]>> = {};
  const addError = (field: RuntimeConfigField, message: string) => {
    fieldErrors[field] = [...(fieldErrors[field] ?? []), message];
  };

  if (!["self-hosted", "client-only", "connect-to-remote", "server-mode"].includes(config.mode)) {
    addError("mode", "请选择有效的运行方式");
  }
  const automaticSelfHostedPort = config.mode === "self-hosted" && config.port === 0;
  if (
    !Number.isInteger(config.port) ||
    config.port < 0 ||
    config.port > 65535 ||
    (config.port === 0 && !automaticSelfHostedPort)
  ) {
    addError("port", "端口必须是 1 到 65535 之间的整数");
  }

  const remote = isRemoteRuntimeMode(config.mode);
  if (remote) {
    if (!config.remoteBaseUrl.trim()) {
      addError("remoteBaseUrl", "请输入远程服务 API 地址");
    } else {
      try {
        normalizeApiBaseUrl(config.remoteBaseUrl);
      } catch (error) {
        addError(
          "remoteBaseUrl",
          error instanceof ApiConfigurationError ? error.message : "远程服务地址无效",
        );
      }
    }
  } else if (!config.bindHost.trim()) {
    addError("bindHost", "请输入本地服务监听地址");
  } else if (!isIpAddress(config.bindHost.trim())) {
    addError("bindHost", "监听地址必须是有效的 IPv4 或 IPv6 地址");
  }

  return {
    valid: Object.keys(fieldErrors).length === 0,
    fieldErrors,
  };
}

function normalizeWebRuntimeConfig(config: EditableRuntimeConfig): EditableRuntimeConfig {
  return config.mode === "self-hosted" && config.port === 0
    ? { ...config, port: defaultRuntimeConfig.port }
    : config;
}

function isIpAddress(value: string): boolean {
  if (value.includes(":")) {
    return /^[0-9a-f:]+$/i.test(value) && value.includes(":");
  }
  const segments = value.split(".");
  return (
    segments.length === 4 &&
    segments.every((segment) => /^\d{1,3}$/.test(segment) && Number(segment) <= 255)
  );
}

type PersistedConfigResult =
  | { status: "missing" }
  | { status: "invalid"; config: EditableRuntimeConfig }
  | { status: "loaded"; config: EditableRuntimeConfig };

function loadPersistedConfig(): PersistedConfigResult {
  let serialized: string | null;
  try {
    serialized = window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return { status: "invalid", config: cloneRuntimeConfig(defaultRuntimeConfig) };
  }
  if (!serialized) {
    return { status: "missing" };
  }

  try {
    const value = JSON.parse(serialized) as unknown;
    return isEditableRuntimeConfig(value)
      ? { status: "loaded", config: value }
      : { status: "invalid", config: cloneRuntimeConfig(defaultRuntimeConfig) };
  } catch {
    return { status: "invalid", config: cloneRuntimeConfig(defaultRuntimeConfig) };
  }
}

function isEditableRuntimeConfig(value: unknown): value is EditableRuntimeConfig {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return (
    typeof record.mode === "string" &&
    typeof record.bindHost === "string" &&
    typeof record.port === "number" &&
    typeof record.remoteBaseUrl === "string"
  );
}

function unsupportedLocalServiceOperation(): RuntimeSnapshot {
  snapshot ??= loadInitialSnapshot();
  return cloneRuntimeSnapshot({
    ...snapshot,
    service: {
      ...snapshot.service,
      error: {
        code: "unsupported_runtime_mode",
        message: "浏览器模式不能直接管理本地 WineStock 服务",
      },
    },
  });
}

function publishSnapshot(): void {
  if (!snapshot) {
    return;
  }
  for (const listener of listeners) {
    listener(cloneRuntimeSnapshot(snapshot));
  }
}

function installResumeListener(): void {
  if (resumeListenerInstalled || typeof document === "undefined") {
    return;
  }
  resumeListenerInstalled = true;
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState !== "visible") {
      return;
    }
    for (const listener of resumeListeners) {
      listener();
    }
  });
}

function normalizeExternalUrl(value: string): string {
  let url: URL;
  try {
    url = new URL(value);
  } catch (error) {
    throw new ApiConfigurationError(`外部链接无效：${String(error)}`);
  }
  if ((url.protocol !== "http:" && url.protocol !== "https:") || url.username || url.password) {
    throw new ApiConfigurationError("外部链接必须使用不含凭据的 http 或 https 地址");
  }
  return url.toString();
}
