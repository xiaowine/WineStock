// 本文件定义 frontend 与 UI 平台 Shell 之间的版本化运行契约；它不实现具体传输或业务 HTTP 请求。

/** shared 当前支持的运行模式。 */
export type RuntimeMode = "self-hosted" | "client-only" | "connect-to-remote" | "server-mode";

/** 当前前端实现支持的 Shell Bridge 协议版本。 */
export const SHELL_BRIDGE_PROTOCOL_VERSION = 1 as const;

/** 注入桥返回不兼容版本或无效快照时使用的稳定错误。 */
export class ShellBridgeContractError extends Error {
  readonly code: "bridge_version_mismatch" | "invalid_bridge_payload";

  constructor(code: "bridge_version_mismatch" | "invalid_bridge_payload", message: string) {
    super(message);
    this.name = "ShellBridgeContractError";
    this.code = code;
  }
}

/** 前端可以编辑并交给 Shell 应用的运行配置。 */
export interface EditableRuntimeConfig {
  /** 决定使用本地 Axum 还是远端服务。 */
  mode: RuntimeMode;
  /** 本地 Axum 监听地址；全接口地址不能直接作为前端访问 URL。 */
  bindHost: string;
  /** 本地 Axum 监听端口。 */
  port: number;
  /** 远端客户端模式使用的 HTTP/HTTPS API 根地址。 */
  remoteBaseUrl: string;
}

/** Shell 配置文件当前是否可用。 */
export type RuntimeConfigStatus = "configured" | "unconfigured" | "invalid";

/** Shell 管理的服务生命周期阶段。 */
export type ShellServicePhase = "stopped" | "starting" | "running" | "stopping" | "failed";

/** 平台向前端返回的稳定运行错误。 */
export interface ShellRuntimeError {
  /** 前端分支使用的稳定错误码。 */
  code: string;
  /** 面向用户的安全错误提示。 */
  message: string;
  /** 错误对应的运行配置字段。 */
  field?: RuntimeConfigField;
}

/** 运行设置表单的稳定字段名称。 */
export type RuntimeConfigField = "mode" | "bindHost" | "port" | "remoteBaseUrl";

/** Shell 当前生效配置、服务状态和平台能力的统一快照。 */
export interface RuntimeSnapshot {
  /** Shell Bridge 协议版本。 */
  protocolVersion: typeof SHELL_BRIDGE_PROTOCOL_VERSION;
  /** 当前宿主平台。 */
  platform: "web" | "desktop" | "android";
  /** 配置文件状态。 */
  configStatus: RuntimeConfigStatus;
  /** 当前生效或待修复的可编辑配置。 */
  config: EditableRuntimeConfig;
  /** Shell 是否已有权威初始化配置；交互式平台通常由成功应用并持久化产生。 */
  initialized: boolean;
  /** 当前服务运行状态。 */
  service: {
    /** 本地服务由 Shell 管理，远端服务只由前端执行 HTTP 检查。 */
    ownership: "local" | "remote";
    /** Shell 观察到的生命周期阶段。 */
    phase: ShellServicePhase;
    /** 前端实际使用的 API 根地址。 */
    apiBaseUrl?: string;
    /** 本地服务真实监听地址，仅用于状态展示。 */
    boundAddress?: string;
    /** Shell 探测到的局域网访问地址。 */
    lanAccessUrls?: string[];
    /** 最近一次配置或生命周期错误。 */
    error?: ShellRuntimeError;
  };
  /** 前端只能依据这些能力决定是否展示平台操作。 */
  capabilities: {
    startLocalService: boolean;
    stopLocalService: boolean;
    restartLocalService: boolean;
    nativeBack: boolean;
    openExternal: boolean;
    serverMode: boolean;
  };
}

/** 配置校验返回的字段错误集合。 */
export interface RuntimeConfigValidationResult {
  /** 全部字段是否通过校验。 */
  valid: boolean;
  /** 按稳定字段名称聚合的错误。 */
  fieldErrors: Partial<Record<RuntimeConfigField, readonly string[]>>;
}

/** 保存并应用运行配置的结果。 */
export interface ApplyRuntimeConfigResult extends RuntimeConfigValidationResult {
  /** Shell 是否成功激活并持久化配置。 */
  applied: boolean;
  /** 成功或失败后 Shell 的权威快照。 */
  snapshot: RuntimeSnapshot;
  /** 非字段运行错误，例如端口占用或本地服务启动失败。 */
  error?: ShellRuntimeError;
}

/** 校验注入桥返回的字段校验结果。 */
export function assertRuntimeConfigValidationResult(
  value: unknown,
): asserts value is RuntimeConfigValidationResult {
  if (!isRecord(value) || typeof value.valid !== "boolean" || !isFieldErrors(value.fieldErrors)) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 返回的配置校验结果无效",
    );
  }
}

/** 校验注入桥返回的配置应用结果及其权威运行快照。 */
export function assertApplyRuntimeConfigResult(
  value: unknown,
): asserts value is ApplyRuntimeConfigResult {
  if (!isRecord(value) || typeof value.applied !== "boolean") {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 返回的配置应用结果无效",
    );
  }
  assertRuntimeConfigValidationResult(value);
  assertCompatibleRuntimeSnapshot(value.snapshot);
  if (value.error !== undefined && !isOptionalRuntimeError(value.error)) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 返回的运行错误结构无效",
    );
  }
}

/** 取消 Shell 状态事件订阅。 */
export type StopShellSubscription = () => void;

/** Android 提交的一次原生返回请求。 */
export interface NativeBackRequest {
  /** 由 Native 页面代次与单调序号组成的一次性标识。 */
  requestId: string;
  /** Android 在提交瞬间观察到的 WebView history 提示。 */
  canGoBack: boolean;
}

/** 前端处理原生返回后的诊断原因；Android 只依据 handled 决定是否 fallback。 */
export type NativeBackReason =
  | "transient-overlay"
  | "image-preview"
  | "dialog"
  | "busy-dialog"
  | "drawer"
  | "popover"
  | "page-state"
  | "route-history"
  | "handler-error"
  | "unhandled";

/** 前端对一次原生返回请求的最终结算。 */
export interface NativeBackResolution {
  requestId: string;
  handled: boolean;
  reason: NativeBackReason;
}

/** Native 对应答的幂等确认；false 表示请求已失效或已经结算。 */
export interface NativeBackResolutionAck {
  accepted: boolean;
}

/** 仅在 capabilities.nativeBack=true 时必须完整存在的 Shell Bridge v1 可选扩展。 */
export interface NativeBackShellBridgeExtension {
  onNativeBackRequested(
    listener: (request: NativeBackRequest) => void,
  ): Promise<StopShellSubscription>;
  resolveNativeBack(resolution: NativeBackResolution): Promise<NativeBackResolutionAck>;
}

/** 前端依赖的统一 Shell Bridge；平台适配层不得扩展为任意 native invoke。 */
export interface ShellBridge {
  /** 读取当前运行配置和服务状态。 */
  getRuntimeSnapshot(): Promise<RuntimeSnapshot>;
  /** 使用 Shell/shared 规则权威校验草稿，不产生副作用。 */
  validateRuntimeConfig(config: EditableRuntimeConfig): Promise<RuntimeConfigValidationResult>;
  /** 保存并应用配置，必要时由 Shell 重启本地服务。 */
  applyRuntimeConfig(config: EditableRuntimeConfig): Promise<ApplyRuntimeConfigResult>;
  /** 启动当前生效配置的本地服务。 */
  startLocalService(): Promise<RuntimeSnapshot>;
  /** 停止当前本地服务。 */
  stopLocalService(): Promise<RuntimeSnapshot>;
  /** 重启当前本地服务。 */
  restartLocalService(): Promise<RuntimeSnapshot>;
  /** 前端首个稳定画面已经渲染。 */
  frontendReady(): Promise<void>;
  /** 通过平台安全能力打开经过校验的外部链接。 */
  openExternal(url: string): Promise<void>;
  /** 订阅配置和服务生命周期快照。 */
  onRuntimeStateChanged(
    listener: (snapshot: RuntimeSnapshot) => void,
  ): Promise<StopShellSubscription>;
  /** 订阅应用从后台恢复事件。 */
  onAppResumed(listener: () => void): Promise<StopShellSubscription>;
  /** capability-gated 的 Android 原生返回事件订阅。 */
  onNativeBackRequested?: NativeBackShellBridgeExtension["onNativeBackRequested"];
  /** capability-gated 的 Android 原生返回应答。 */
  resolveNativeBack?: NativeBackShellBridgeExtension["resolveNativeBack"];
}

/** 初始快照版本通过后，确认注入桥完整实现 v1 所有具名方法。 */
export function assertCompleteShellBridge(value: unknown): asserts value is ShellBridge {
  if (!isRecord(value)) {
    throw new ShellBridgeContractError("invalid_bridge_payload", "平台注入的 Shell Bridge 无效");
  }
  const requiredMethods: ReadonlyArray<keyof ShellBridge> = [
    "getRuntimeSnapshot",
    "validateRuntimeConfig",
    "applyRuntimeConfig",
    "startLocalService",
    "stopLocalService",
    "restartLocalService",
    "frontendReady",
    "openExternal",
    "onRuntimeStateChanged",
    "onAppResumed",
  ];
  const missing = requiredMethods.filter((method) => typeof value[method] !== "function");
  if (missing.length > 0) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      `Shell Bridge v1 缺少方法：${missing.join("、")}`,
    );
  }
}

/** nativeBack capability 开启后，收窄并校验两个可选扩展方法。 */
export function assertNativeBackShellBridgeExtension(
  value: ShellBridge,
): asserts value is ShellBridge & NativeBackShellBridgeExtension {
  if (
    typeof value.onNativeBackRequested !== "function" ||
    typeof value.resolveNativeBack !== "function"
  ) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 声明 nativeBack 能力但缺少订阅或应答方法",
    );
  }
}

/** 校验 Native 发布的原生返回请求，避免无效事件进入 UI handler registry。 */
export function assertNativeBackRequest(value: unknown): asserts value is NativeBackRequest {
  if (
    !isRecord(value) ||
    typeof value.requestId !== "string" ||
    value.requestId.length < 1 ||
    value.requestId.length > 64 ||
    typeof value.canGoBack !== "boolean"
  ) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 发布了无效的原生返回请求",
    );
  }
}

/** 校验 Native 对原生返回应答的幂等确认。 */
export function assertNativeBackResolutionAck(
  value: unknown,
): asserts value is NativeBackResolutionAck {
  if (!isRecord(value) || typeof value.accepted !== "boolean") {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 返回了无效的原生返回确认",
    );
  }
}

/** shared 默认配置在前端表单中的稳定镜像。 */
export const defaultRuntimeConfig: EditableRuntimeConfig = {
  mode: "self-hosted",
  bindHost: "127.0.0.1",
  port: 17890,
  remoteBaseUrl: "",
};

/** 创建可安全修改的配置副本，避免页面直接改写 Shell 快照。 */
export function cloneRuntimeConfig(config: EditableRuntimeConfig): EditableRuntimeConfig {
  return {
    mode: config.mode,
    bindHost: config.bindHost,
    port: config.port,
    remoteBaseUrl: config.remoteBaseUrl,
  };
}

/** 创建包含数组副本的运行快照，避免跨适配层共享可变引用。 */
export function cloneRuntimeSnapshot(snapshot: RuntimeSnapshot): RuntimeSnapshot {
  return {
    ...snapshot,
    config: cloneRuntimeConfig(snapshot.config),
    service: {
      ...snapshot.service,
      lanAccessUrls: snapshot.service.lanAccessUrls
        ? [...snapshot.service.lanAccessUrls]
        : undefined,
      error: snapshot.service.error ? { ...snapshot.service.error } : undefined,
    },
    capabilities: { ...snapshot.capabilities },
  };
}

/** 在使用注入桥数据前校验协议版本和运行快照基础结构。 */
export function assertCompatibleRuntimeSnapshot(value: unknown): asserts value is RuntimeSnapshot {
  if (!isRecord(value)) {
    throw new ShellBridgeContractError("invalid_bridge_payload", "Shell Bridge 返回了无效运行快照");
  }
  if (value.protocolVersion !== SHELL_BRIDGE_PROTOCOL_VERSION) {
    throw new ShellBridgeContractError(
      "bridge_version_mismatch",
      `Shell Bridge 协议版本不兼容：前端需要 v${SHELL_BRIDGE_PROTOCOL_VERSION}，Shell 返回 ${String(value.protocolVersion ?? "缺失")}`,
    );
  }

  const config = value.config;
  const service = value.service;
  const capabilities = value.capabilities;
  if (
    !["web", "desktop", "android"].includes(String(value.platform)) ||
    !["configured", "unconfigured", "invalid"].includes(String(value.configStatus)) ||
    typeof value.initialized !== "boolean" ||
    !isEditableRuntimeConfig(config) ||
    !isRecord(service) ||
    !["local", "remote"].includes(String(service.ownership)) ||
    !["stopped", "starting", "running", "stopping", "failed"].includes(String(service.phase)) ||
    !isOptionalString(service.apiBaseUrl) ||
    !isOptionalString(service.boundAddress) ||
    !isOptionalStringArray(service.lanAccessUrls) ||
    !isOptionalRuntimeError(service.error) ||
    !isRecord(capabilities) ||
    ![
      capabilities.startLocalService,
      capabilities.stopLocalService,
      capabilities.restartLocalService,
      capabilities.nativeBack,
      capabilities.openExternal,
      capabilities.serverMode,
    ].every((item) => typeof item === "boolean")
  ) {
    throw new ShellBridgeContractError(
      "invalid_bridge_payload",
      "Shell Bridge 返回的运行快照结构无效",
    );
  }
}

function isEditableRuntimeConfig(value: unknown): value is EditableRuntimeConfig {
  if (!isRecord(value)) return false;
  return (
    ["self-hosted", "client-only", "connect-to-remote", "server-mode"].includes(
      String(value.mode),
    ) &&
    typeof value.bindHost === "string" &&
    typeof value.port === "number" &&
    typeof value.remoteBaseUrl === "string"
  );
}

function isOptionalRuntimeError(value: unknown): boolean {
  if (value === undefined) return true;
  return (
    isRecord(value) &&
    typeof value.code === "string" &&
    typeof value.message === "string" &&
    (value.field === undefined ||
      ["mode", "bindHost", "port", "remoteBaseUrl"].includes(String(value.field)))
  );
}

function isFieldErrors(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return Object.entries(value).every(
    ([field, messages]) =>
      ["mode", "bindHost", "port", "remoteBaseUrl"].includes(field) &&
      Array.isArray(messages) &&
      messages.every((message) => typeof message === "string"),
  );
}

function isOptionalString(value: unknown): boolean {
  return value === undefined || typeof value === "string";
}

function isOptionalStringArray(value: unknown): boolean {
  return (
    value === undefined || (Array.isArray(value) && value.every((item) => typeof item === "string"))
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
