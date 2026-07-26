// 本文件拥有出入库草稿工作台物品目录的搜索、取消、分页合并和滚动触底状态；它不管理草稿明细。
import { computed, onBeforeUnmount, ref } from "vue";
import { listItemOptions, type ItemOptionResponse } from "../api/items";

const pageSize = 50;

/** 服务端分页物品目录。 */
export function useStockItemCatalog(errorMessage: (error: unknown) => string) {
  const items = ref<ItemOptionResponse[]>([]);
  const totalItems = ref(0);
  const currentPage = ref(0);
  const totalPages = ref(0);
  const activeSearch = ref("");
  const searchInput = ref("");
  const loadingItems = ref(false);
  const itemError = ref("");
  const itemList = ref<HTMLElement | null>(null);
  let controller: AbortController | null = null;
  let requestGeneration = 0;

  const exhausted = computed(() => currentPage.value > 0 && currentPage.value >= totalPages.value);
  const resultLabel = computed(() =>
    loadingItems.value && items.value.length === 0
      ? "正在加载结果"
      : `共 ${totalItems.value} 个物品`,
  );

  onBeforeUnmount(() => controller?.abort());

  async function reset(): Promise<void> {
    requestGeneration += 1;
    controller?.abort();
    controller = null;
    loadingItems.value = false;
    items.value = [];
    currentPage.value = 0;
    totalPages.value = 0;
    totalItems.value = 0;
    itemError.value = "";
    await loadNext();
  }

  async function loadNext(): Promise<void> {
    if (loadingItems.value || exhausted.value) return;
    const generation = requestGeneration;
    const nextPage = currentPage.value + 1;
    const request = new AbortController();
    controller = request;
    loadingItems.value = true;
    itemError.value = "";
    try {
      const response = await listItemOptions(
        activeSearch.value,
        nextPage,
        pageSize,
        request.signal,
      );
      if (generation !== requestGeneration) return;
      const merged = new Map(items.value.map((item) => [item.id, item]));
      response.items.forEach((item) => merged.set(item.id, item));
      items.value = Array.from(merged.values());
      totalItems.value = response.total;
      currentPage.value = response.page;
      totalPages.value = response.total_pages;
    } catch (error) {
      if (
        generation === requestGeneration &&
        !(error instanceof DOMException && error.name === "AbortError")
      ) {
        itemError.value = errorMessage(error);
      }
    } finally {
      if (controller === request) {
        controller = null;
        loadingItems.value = false;
      }
    }
  }

  function applySearch(value: string): void {
    if (value === activeSearch.value) return;
    activeSearch.value = value;
    void reset();
  }

  function handleScroll(): void {
    const element = itemList.value;
    if (element && element.scrollHeight - element.scrollTop - element.clientHeight < 160)
      void loadNext();
  }

  return {
    items,
    totalItems,
    searchInput,
    loadingItems,
    itemError,
    itemList,
    itemsExhausted: exhausted,
    itemResultLabel: resultLabel,
    resetItems: reset,
    loadNextItems: loadNext,
    applySearch,
    handleItemScroll: handleScroll,
  };
}
