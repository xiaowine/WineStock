// 本文件把平台无关的原生返回 core 接到 Vue Router 与 Shell Bridge；它不拥有具体浮层状态。
import type { Router } from "vue-router";
import type { NativeBackRequest } from "../shell/contract";
import { resolveNativeBack, subscribeNativeBackRequested } from "../shell/runtime";
import {
  createNativeBackRegistry,
  installNativeBackCoordinator,
  type NativeBackHandlerRegistration,
  type NativeBackHandlerResult,
} from "./nativeBackCore";

export { NativeBackPriority } from "./nativeBackCore";
export type {
  NativeBackHandlerContext,
  NativeBackHandlerRegistration,
  NativeBackHandlerResult,
} from "./nativeBackCore";

const nativeBackRegistry = createNativeBackRegistry({
  onHandlerError: (registration, error) => {
    console.warn(`原生返回 handler 执行失败：${registration.id}`, error);
  },
});

/** 注册一个当前激活的 handler；取消函数幂等，并由 composable 在关闭或卸载时调用。 */
export function registerNativeBackHandler(registration: NativeBackHandlerRegistration): () => void {
  return nativeBackRegistry.register(registration);
}

/** 按 priority 降序、同级最近激活优先调度，直到第一个 handler 消费请求。 */
export function dispatchNativeBackRequest(
  request: NativeBackRequest,
): Promise<NativeBackHandlerResult> {
  return nativeBackRegistry.dispatch(request);
}

/**
 * 在 Vue 挂载后安装 Android 订阅，并注册优先级最低的 Vue Router history handler。
 * router.back() 提交后立即报告 handled，不等待可能打开异步确认框的离开守卫完成。
 */
export async function installNativeBackNavigation(router: Router): Promise<() => void> {
  return installNativeBackCoordinator({
    registry: nativeBackRegistry,
    subscribe: subscribeNativeBackRequested,
    resolve: resolveNativeBack,
    navigateBack: () => router.back(),
    onResolutionError: (request, error) => {
      console.warn(`无法结算原生返回请求 ${request.requestId}`, error);
    },
  });
}
