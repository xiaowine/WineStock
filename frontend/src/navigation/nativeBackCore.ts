// 本文件拥有平台无关的原生返回 registry 与一次性请求协调；它不依赖 Vue、Router 实例或 Shell Bridge 传输。
import type { NativeBackReason, NativeBackRequest, NativeBackResolution } from "../shell/contract";

/** 临时 UI 到路由 history 的稳定处理优先级。 */
export const NativeBackPriority = {
  TransientOverlay: 500,
  ImagePreview: 450,
  Dialog: 400,
  Drawer: 300,
  Popover: 300,
  PageState: 200,
  RouteHistory: 100,
} as const;

export interface NativeBackHandlerContext {
  request: NativeBackRequest;
}

export type NativeBackHandlerResult =
  { handled: true; reason: NativeBackReason } | { handled: false };

export interface NativeBackHandlerRegistration {
  /** 仅用于诊断；同类组件可以使用带 Vue uid 的实例 id。 */
  id: string;
  priority: number;
  isActive(): boolean;
  handle(
    context: NativeBackHandlerContext,
  ): NativeBackHandlerResult | Promise<NativeBackHandlerResult>;
}

export interface NativeBackRegistry {
  register(registration: NativeBackHandlerRegistration): () => void;
  dispatch(request: NativeBackRequest): Promise<NativeBackHandlerResult>;
  clear(): void;
}

export interface NativeBackRegistryOptions {
  onHandlerError?(registration: NativeBackHandlerRegistration, error: unknown): void;
}

interface RegisteredHandler {
  token: number;
  activationSequence: number;
  registration: NativeBackHandlerRegistration;
}

/** 创建隔离的 handler registry；生产环境使用单例，测试可为每个场景创建独立实例。 */
export function createNativeBackRegistry(
  options: NativeBackRegistryOptions = {},
): NativeBackRegistry {
  let nextHandlerToken = 1;
  let nextActivationSequence = 1;
  const registeredHandlers = new Map<number, RegisteredHandler>();

  return {
    register(registration) {
      const token = nextHandlerToken++;
      registeredHandlers.set(token, {
        token,
        activationSequence: nextActivationSequence++,
        registration,
      });
      return () => registeredHandlers.delete(token);
    },

    async dispatch(request) {
      const candidates = Array.from(registeredHandlers.values()).sort(
        (left, right) =>
          right.registration.priority - left.registration.priority ||
          right.activationSequence - left.activationSequence,
      );

      for (const candidate of candidates) {
        if (!registeredHandlers.has(candidate.token)) continue;
        try {
          if (!candidate.registration.isActive()) continue;
          const result = await candidate.registration.handle({ request });
          if (result.handled) return result;
        } catch (error) {
          options.onHandlerError?.(candidate.registration, error);
          return { handled: true, reason: "handler-error" };
        }
      }

      return { handled: false };
    },

    clear() {
      registeredHandlers.clear();
    },
  };
}

export interface NativeBackCoordinatorOptions {
  registry: NativeBackRegistry;
  subscribe(listener: (request: NativeBackRequest) => void): Promise<() => void>;
  resolve(resolution: NativeBackResolution): Promise<unknown>;
  navigateBack(): void;
  onResolutionError?(request: NativeBackRequest, error: unknown): void;
}

/**
 * 安装一次 Native 事件协调：注册最低优先级的路由 handler、去重 requestId，并确保每个事件至多应答一次。
 */
export async function installNativeBackCoordinator(
  options: NativeBackCoordinatorOptions,
): Promise<() => void> {
  let disposed = false;
  const processedRequestIds = new Set<string>();
  const unregisterRouteHandler = options.registry.register({
    id: "vue-router-history",
    priority: NativeBackPriority.RouteHistory,
    isActive: () => true,
    handle: ({ request }) => {
      if (!request.canGoBack) return { handled: false };
      options.navigateBack();
      return { handled: true, reason: "route-history" };
    },
  });

  let stopSubscription: () => void;
  try {
    stopSubscription = await options.subscribe((request) => {
      if (disposed || processedRequestIds.has(request.requestId)) return;
      rememberProcessedRequest(processedRequestIds, request.requestId);
      void settleNativeBackRequest(options, request, () => disposed);
    });
  } catch (error) {
    unregisterRouteHandler();
    throw error;
  }

  return () => {
    if (disposed) return;
    disposed = true;
    stopSubscription();
    unregisterRouteHandler();
    processedRequestIds.clear();
    options.registry.clear();
  };
}

async function settleNativeBackRequest(
  options: NativeBackCoordinatorOptions,
  request: NativeBackRequest,
  isDisposed: () => boolean,
): Promise<void> {
  const result = await options.registry.dispatch(request);
  if (isDisposed()) return;
  try {
    await options.resolve({
      requestId: request.requestId,
      handled: result.handled,
      reason: result.handled ? result.reason : "unhandled",
    });
  } catch (error) {
    options.onResolutionError?.(request, error);
  }
}

/** requestId 单调递增，仅保留小型诊断窗口，防止异常宿主重复事件导致集合无限增长。 */
function rememberProcessedRequest(processed: Set<string>, requestId: string): void {
  processed.add(requestId);
  if (processed.size <= 64) return;
  const oldest = processed.values().next().value;
  if (typeof oldest === "string") processed.delete(oldest);
}
