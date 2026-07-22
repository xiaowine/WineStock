// 本文件把 Vue 响应式临时状态接入全局原生返回 registry；它不决定业务优先级或关闭行为。
import { toValue, watch, type MaybeRefOrGetter } from "vue";
import {
  registerNativeBackHandler,
  type NativeBackHandlerRegistration,
} from "../navigation/nativeBack";

export interface UseNativeBackHandlerOptions extends Omit<
  NativeBackHandlerRegistration,
  "isActive"
> {
  /** false 时不占用 registry；再次变 true 会重新注册，从而刷新同级 LIFO 顺序。 */
  active: MaybeRefOrGetter<boolean>;
}

/** 随 active 注册/注销 handler；watch 清理会在状态变化和组件作用域销毁时自动执行。 */
export function useNativeBackHandler(options: UseNativeBackHandlerOptions): void {
  watch(
    () => toValue(options.active),
    (active, _previous, onCleanup) => {
      if (!active) return;
      const unregister = registerNativeBackHandler({
        id: options.id,
        priority: options.priority,
        isActive: () => toValue(options.active),
        handle: options.handle,
      });
      onCleanup(unregister);
    },
    { immediate: true, flush: "sync" },
  );
}
