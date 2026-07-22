// 本文件拥有 frontend 原生返回 handler 注册表、优先级、LIFO 调度与 Vue Router 最终 fallback；它不实现平台传输或具体浮层状态。
import type { Router } from "vue-router";
import type { NativeBackReason, NativeBackRequest } from "../shell/contract";
import { resolveNativeBack, subscribeNativeBackRequested } from "../shell/runtime";

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

interface RegisteredHandler {
  token: number;
  activationSequence: number;
  registration: NativeBackHandlerRegistration;
}

let nextHandlerToken = 1;
let nextActivationSequence = 1;
const registeredHandlers = new Map<number, RegisteredHandler>();

/** 注册一个当前激活的 handler；取消函数幂等，并由 composable 在关闭或卸载时调用。 */
export function registerNativeBackHandler(registration: NativeBackHandlerRegistration): () => void {
  const token = nextHandlerToken++;
  registeredHandlers.set(token, {
    token,
    activationSequence: nextActivationSequence++,
    registration,
  });
  return () => registeredHandlers.delete(token);
}

/** 按 priority 降序、同级最近激活优先调度，直到第一个 handler 消费请求。 */
export async function dispatchNativeBackRequest(
  request: NativeBackRequest,
): Promise<NativeBackHandlerResult> {
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
      console.warn(`原生返回 handler 执行失败：${candidate.registration.id}`, error);
      return { handled: true, reason: "handler-error" };
    }
  }

  return { handled: false };
}

/**
 * 在 Vue 挂载后安装 Android 订阅，并注册优先级最低的 Vue Router history handler。
 * router.back() 提交后立即报告 handled，不等待可能打开异步确认框的离开守卫完成。
 */
export async function installNativeBackNavigation(router: Router): Promise<() => void> {
  let disposed = false;
  const processedRequestIds = new Set<string>();
  const unregisterRouteHandler = registerNativeBackHandler({
    id: "vue-router-history",
    priority: NativeBackPriority.RouteHistory,
    isActive: () => true,
    handle: ({ request }) => {
      if (!request.canGoBack) return { handled: false };
      router.back();
      return { handled: true, reason: "route-history" };
    },
  });

  let stopSubscription: () => void;
  try {
    stopSubscription = await subscribeNativeBackRequested((request) => {
      if (disposed || processedRequestIds.has(request.requestId)) return;
      rememberProcessedRequest(processedRequestIds, request.requestId);
      void settleNativeBackRequest(request, () => disposed);
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
    registeredHandlers.clear();
  };
}

async function settleNativeBackRequest(
  request: NativeBackRequest,
  isDisposed: () => boolean,
): Promise<void> {
  const result = await dispatchNativeBackRequest(request);
  if (isDisposed()) return;
  try {
    await resolveNativeBack({
      requestId: request.requestId,
      handled: result.handled,
      reason: result.handled ? result.reason : "unhandled",
    });
  } catch (error) {
    console.warn(`无法结算原生返回请求 ${request.requestId}`, error);
  }
}

/** requestId 单调递增，仅保留小型诊断窗口，防止异常宿主重复事件导致集合无限增长。 */
function rememberProcessedRequest(processed: Set<string>, requestId: string): void {
  processed.add(requestId);
  if (processed.size <= 64) return;
  const oldest = processed.values().next().value;
  if (typeof oldest === "string") processed.delete(oldest);
}
