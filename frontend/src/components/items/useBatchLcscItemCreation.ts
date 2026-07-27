// 本文件拥有立创物品一键批量创建会话：按 C 号串行查资料、拉图、套模板并创建物品；
// 它不拥有预览列表 UI，也不决定批量入口所在的业务流程（订单导入/备份导入共用）。
// 设计见 docs/implementation-notes/lcsc-batch-item-creation-and-erp-backup-import.md。
import { computed, onBeforeUnmount, ref } from "vue";
import { ApiError } from "../../api/errors";
import {
  createItem,
  listItemOptions,
  lookupLcscItem,
  type ItemOptionResponse,
  type LcscItemLookupResponse,
} from "../../api/items";
import { prepareLcscItemImage } from "./lcscImageDraft";
import { listItemCategories, type ItemCategoryResponse } from "../../api/itemCategories";
import {
  listItemAttributeTemplates,
  type ItemAttributeTemplateResponse,
} from "../../api/itemAttributeTemplates";
import {
  applyLcscLookupToDraft,
  defaultAttributeTemplate,
  emptyItemDraft,
  itemCreateRequest,
  validateItemDraft,
} from "../../pages/items/model";
import { discardTemporaryItemFiles } from "../../pages/items/fileCleanup";
import {
  createPendingImageDraft,
  isImageDraftValue,
  releaseImageDraft,
  uploadImageDrafts,
} from "../attributes/imageDraft";

/** 批次级创建选项；整批一个模板，个别物品事后精调。 */
export interface BatchLcscCreationOptions {
  templateId: number | null;
  categoryId: number | null;
  unit: string;
}

/** 单个 C 号的批量创建结果。 */
export type BatchLcscItemResult =
  | { ok: true; item: ItemOptionResponse }
  | { ok: false; reason: string }
  | { ok: false; cancelled: true; reason: "" };

export interface BatchLcscRunCallbacks {
  /** 某 C 号开始创建（用于行状态转"创建中"）。 */
  onItemStarted?: (code: string) => void;
  /** 某 C 号创建成功或确认已存在（自动匹配）。 */
  onItemCreated?: (code: string, item: ItemOptionResponse) => void;
  /** 某 C 号创建失败及可见原因。 */
  onItemFailed?: (code: string, reason: string) => void;
}

/**
 * 创建可复用的批量创建会话。串行执行（同时天然限制对立创上游的请求频率），
 * 单项失败不阻塞后续；同 C 号去重，`sku_taken` 视为已存在并自动匹配。
 */
export function useBatchLcscItemCreation() {
  const templates = ref<ItemAttributeTemplateResponse[]>([]);
  const categories = ref<ItemCategoryResponse[]>([]);
  const metadataLoading = ref(false);
  const metadataError = ref("");
  const running = ref(false);
  const progressDone = ref(0);
  const progressTotal = ref(0);
  /** 批次选项在同一会话内记住，连续批量创建不必重选；不做持久化。 */
  const options = ref<BatchLcscCreationOptions | null>(null);
  let metadataController: AbortController | null = null;
  let runController: AbortController | null = null;
  let metadataLoaded = false;

  const progressLabel = computed(() =>
    running.value ? `${progressDone.value}/${progressTotal.value}` : "",
  );

  onBeforeUnmount(() => {
    metadataController?.abort();
    runController?.abort();
  });

  /** 加载模板与分类供批量选项对话框使用；重复调用复用已加载结果。 */
  async function loadMetadata(): Promise<boolean> {
    if (metadataLoaded) return true;
    metadataController?.abort();
    const controller = new AbortController();
    metadataController = controller;
    metadataLoading.value = true;
    metadataError.value = "";
    try {
      const [nextTemplates, nextCategories] = await Promise.all([
        listItemAttributeTemplates(controller.signal),
        listItemCategories(controller.signal),
      ]);
      if (metadataController !== controller) return false;
      templates.value = nextTemplates;
      categories.value = nextCategories;
      metadataLoaded = true;
      return true;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return false;
      metadataError.value =
        error instanceof ApiError ? error.message : "无法加载模板与分类，请重试";
      return false;
    } finally {
      if (metadataController === controller) {
        metadataController = null;
        metadataLoading.value = false;
      }
    }
  }

  /** 返回默认批次选项：模板预选全站默认，分类不指定，单位"个"。 */
  function defaultOptions(): BatchLcscCreationOptions {
    return (
      options.value ?? {
        templateId: defaultAttributeTemplate(templates.value)?.id ?? null,
        categoryId: null,
        unit: "个",
      }
    );
  }

  /** 串行批量创建；codes 去重后逐个执行，结束返回成功数。 */
  async function run(
    codes: readonly string[],
    nextOptions: BatchLcscCreationOptions,
    callbacks: BatchLcscRunCallbacks = {},
  ): Promise<number> {
    if (running.value) return 0;
    options.value = { ...nextOptions, unit: nextOptions.unit.trim() || "个" };
    const uniqueCodes = [...new Set(codes.map((code) => code.trim().toUpperCase()))].filter(
      Boolean,
    );
    const controller = new AbortController();
    runController = controller;
    running.value = true;
    progressDone.value = 0;
    progressTotal.value = uniqueCodes.length;
    let createdCount = 0;
    try {
      for (const code of uniqueCodes) {
        if (controller.signal.aborted) break;
        callbacks.onItemStarted?.(code);
        const result = await createOne(code, options.value, controller.signal);
        progressDone.value += 1;
        if ("cancelled" in result) break;
        if (result.ok) {
          createdCount += 1;
          callbacks.onItemCreated?.(code, result.item);
        } else {
          callbacks.onItemFailed?.(code, result.reason);
        }
      }
    } finally {
      running.value = false;
      if (runController === controller) runController = null;
    }
    return createdCount;
  }

  /** 中止当前批量执行；已创建的物品保留，进行中的项回到失败/待处理由调用方决定。 */
  function cancel(): void {
    runController?.abort();
  }

  async function createOne(
    code: string,
    batchOptions: BatchLcscCreationOptions,
    signal: AbortSignal,
  ): Promise<BatchLcscItemResult> {
    const template =
      templates.value.find((candidate) => candidate.id === batchOptions.templateId) ?? null;
    const draft = emptyItemDraft();
    draft.unit = batchOptions.unit;
    draft.categoryId = batchOptions.categoryId;

    let candidate: LcscItemLookupResponse;
    try {
      candidate = await lookupLcscItem(code, signal);
      applyLcscLookupToDraft(draft, candidate, template);
    } catch (error) {
      if (isAbort(error, signal)) return cancelledResult();
      return {
        ok: false,
        reason:
          error instanceof ApiError ? `立创资料查询失败：${error.message}` : "立创资料查询失败",
      };
    }

    // 主图为必填字段：Core 无候选或选定图片读取失败时统一生成可识别占位图。
    try {
      const image = await prepareLcscItemImage(candidate.image_url, code, signal);
      if (signal.aborted) return cancelledResult();
      draft.image = createPendingImageDraft(image.file);
      draft.imageTemporary = true;
    } catch (error) {
      if (isAbort(error, signal)) return cancelledResult();
      return { ok: false, reason: "商品图片获取失败（物品主图为必填）" };
    }

    const validation = validateItemDraft(draft, templates.value);
    if (validation) {
      releaseImageDraft(draft.image ?? undefined);
      // 常见于模板必填字段没有对应的立创参数；批量无法代填，留给单个新建处理。
      return { ok: false, reason: `资料不满足校验：${validation.firstMessage}` };
    }

    try {
      await uploadImageDrafts([
        draft.image as NonNullable<typeof draft.image>,
        ...draft.attributes.map((attribute) => attribute.value).filter(isImageDraftValue),
      ]);
      const result = await createItem(itemCreateRequest(draft));
      const created = await findItemOption(code, result.id, signal);
      if (!created) return { ok: false, reason: "创建成功但未能读取物品，请重试匹配" };
      return { ok: true, item: created };
    } catch (error) {
      if (isAbort(error, signal)) return cancelledResult();
      if (error instanceof ApiError && error.code === "sku_taken") {
        // 并发或既有数据已占用该编号：视为已存在，直接匹配。
        await cleanupDraft();
        const existing = await findItemOption(code, null, signal).catch(() => null);
        if (existing) return { ok: true, item: existing };
        return { ok: false, reason: "编号已存在但未能匹配到物品，请重试匹配" };
      }
      await cleanupDraft();
      return {
        ok: false,
        reason: error instanceof ApiError ? `创建失败：${error.message}` : "创建失败，请重试",
      };
    }

    async function cleanupDraft(): Promise<void> {
      try {
        await discardTemporaryItemFiles(draft);
      } catch {
        // 临时图片超过保留期后由服务端自动清理。
      }
    }
  }

  /** 创建后按编号读取业务选择器物品；id 提供时优先精确匹配。 */
  async function findItemOption(
    code: string,
    id: number | null,
    signal: AbortSignal,
  ): Promise<ItemOptionResponse | null> {
    const page = await listItemOptions(code, 1, 20, signal);
    return (
      page.items.find((item) =>
        id !== null ? item.id === id : item.sku.trim().toUpperCase() === code,
      ) ?? null
    );
  }

  return {
    templates,
    categories,
    metadataLoading,
    metadataError,
    running,
    progressDone,
    progressTotal,
    progressLabel,
    loadMetadata,
    defaultOptions,
    run,
    cancel,
  };
}

function isAbort(error: unknown, signal: AbortSignal): boolean {
  return signal.aborted || (error instanceof DOMException && error.name === "AbortError");
}

function cancelledResult(): BatchLcscItemResult {
  return { ok: false, cancelled: true, reason: "" };
}
