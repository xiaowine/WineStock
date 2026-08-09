// 本文件拥有立创资料查询请求的执行状态：竞态取消、错误文案映射和候选结果；它不拥有任何 Dialog 布局或草稿写入。
import { onScopeDispose, ref } from "vue";
import { ApiError, ApiNetworkError } from "../../api/errors";
import { translateMessageOrNull } from "../../i18n";
import { lookupLcscItem, type LcscItemLookupResponse } from "../../api/items";

/** 供手动查询 Dialog 与扫码查询流共享的单次查询状态机。 */
export function useLcscLookupRequest() {
  const pending = ref(false);
  const error = ref("");
  const candidate = ref<LcscItemLookupResponse | null>(null);
  let controller: AbortController | null = null;
  let generation = 0;

  /** 执行一次查询；被新请求或 abort 取代的旧请求不写回任何状态。 */
  async function lookup(productCode: string): Promise<void> {
    abort();
    const requestController = new AbortController();
    const requestGeneration = ++generation;
    controller = requestController;
    pending.value = true;
    error.value = "";
    candidate.value = null;
    try {
      const result = await lookupLcscItem(productCode, requestController.signal);
      if (requestGeneration !== generation) return;
      candidate.value = result;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      if (requestGeneration === generation) error.value = lcscLookupErrorMessage(cause);
    } finally {
      if (controller === requestController) {
        controller = null;
        pending.value = false;
      }
    }
  }

  /** 中止进行中的请求并清空全部状态。 */
  function reset(): void {
    abort();
    error.value = "";
    candidate.value = null;
  }

  function abort(): void {
    generation += 1;
    controller?.abort();
    controller = null;
    pending.value = false;
  }

  onScopeDispose(abort);

  return { pending, error, candidate, lookup, reset, abort };
}

/** 把立创查询错误映射为面向用户的稳定文案。 */
export function lcscLookupErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    // 码表已由 error.<code> 语言包覆盖；未命中时回退后端已本地化的 message。
    return translateMessageOrNull(`error.${error.code}`) ?? error.message;
  }
  if (error instanceof ApiNetworkError) {
    return translateMessageOrNull("error.network_unavailable") ?? "";
  }
  return translateMessageOrNull("items.lcscLookupUnknownError") ?? "";
}
