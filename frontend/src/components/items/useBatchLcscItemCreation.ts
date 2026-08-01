// 本文件拥有立创物品一键批量创建会话：按 C 号查询资料后立即拉图、套模板并创建物品；
// 它不拥有预览列表 UI，也不决定批量入口所在的业务流程（订单导入/备份导入共用）。
// 设计见 docs/implementation-notes/lcsc-batch-item-creation-and-erp-backup-import.md。
import { computed, onBeforeUnmount, ref } from "vue";
import { ApiError } from "../../api/errors";
import {
  createItem,
  listItemOptions,
  lookupItemOptions,
  lookupLcscItems,
  type LcscBatchLookupError,
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

const LCSC_BATCH_LOOKUP_SIZE = 10;

/** 批次级创建选项；整批一个模板，个别物品事后精调。 */
export interface BatchLcscCreationOptions {
  templateId: number | null;
  categoryId: number | null;
  unit: string;
}

/** 单个 C 号的批量创建结果。 */
export type BatchLcscItemResult =
  | { ok: true; item: ItemOptionResponse; created: boolean }
  | { ok: false; reason: string }
  | { ok: false; cancelled: true; reason: "" };

export interface BatchLcscRunCallbacks {
  /** 某 C 号开始查询立创资料。 */
  onItemLookupStarted?: (code: string) => void;
  /** 某 C 号开始创建（用于行状态转"创建中"）。 */
  onItemStarted?: (code: string) => void;
  /** 某 C 号已就绪；created 为 false 时表示本地或并发会话已存在。 */
  onItemCreated?: (code: string, item: ItemOptionResponse, created: boolean) => void;
  /** 某 C 号创建失败及可见原因。 */
  onItemFailed?: (code: string, reason: string) => void;
}

/**
 * 创建可复用的批量创建会话。每个 C 号按“查询资料 -> 创建物品”顺序执行，
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
  const progressPhase = ref("准备本地物品");
  /** 批次选项在同一会话内记住，连续批量创建不必重选；不做持久化。 */
  const options = ref<BatchLcscCreationOptions | null>(null);
  let metadataController: AbortController | null = null;
  let runController: AbortController | null = null;
  let metadataLoaded = false;

  const progressLabel = computed(() =>
    running.value ? `${progressPhase.value} ${progressDone.value}/${progressTotal.value}` : "",
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

  /** 按客编每 10 个批量查询立创资料，批次返回后逐个创建；codes 去重后执行。 */
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
    progressPhase.value = "检查本地物品";
    let createdCount = 0;
    try {
      let pendingCodes = uniqueCodes;
      try {
        // 创建前再次确认本地 SKU，避免匹配完成后被其他会话创建的物品重复走立创资料和图片流程。
        const existing = await lookupItemOptions(uniqueCodes, controller.signal);
        const existingCodes = new Set<string>();
        for (const result of existing.results) {
          if (result.item) {
            existingCodes.add(normalizeProductCode(result.product_code));
            progressDone.value += 1;
            callbacks.onItemCreated?.(result.product_code, result.item, false);
          }
        }
        pendingCodes = uniqueCodes.filter((code) => !existingCodes.has(code));
      } catch (error) {
        if (isAbort(error, controller.signal)) return 0;
        const reason =
          error instanceof ApiError
            ? `无法确认本地物品：${error.message}`
            : "无法确认本地物品，请重试";
        for (const code of uniqueCodes) {
          progressDone.value += 1;
          callbacks.onItemFailed?.(code, reason);
        }
        return 0;
      }

      for (let start = 0; start < pendingCodes.length; start += LCSC_BATCH_LOOKUP_SIZE) {
        if (controller.signal.aborted) return 0;
        const batchCodes = pendingCodes.slice(start, start + LCSC_BATCH_LOOKUP_SIZE);
        for (const code of batchCodes) callbacks.onItemLookupStarted?.(code);
        progressPhase.value = "查询立创资料";
        let lookupResults: Awaited<ReturnType<typeof lookupLcscItems>>;
        try {
          lookupResults = await lookupLcscItems(batchCodes, controller.signal);
        } catch (error) {
          if (isAbort(error, controller.signal)) return 0;
          const reason =
            error instanceof ApiError
              ? `立创资料查询失败：${error.message}`
              : "立创资料查询失败，请重试";
          for (const code of batchCodes) {
            progressDone.value += 1;
            callbacks.onItemFailed?.(code, reason);
          }
          continue;
        }

        // 批量响应到达后逐个落地，避免等待整批创建完成才更新列表。
        // 按请求批次取结果，缺失结果也要明确结束该行，避免界面永久停留在查询中。
        const resultByCode = new Map(
          lookupResults.results.map((result) => [
            normalizeProductCode(result.product_code),
            result,
          ]),
        );
        for (const code of batchCodes) {
          if (controller.signal.aborted) return 0;
          const result = resultByCode.get(code);
          if (!result) {
            progressDone.value += 1;
            callbacks.onItemFailed?.(code, lookupFailureMessage("invalid_response"));
            continue;
          }
          if (!result.candidate) {
            progressDone.value += 1;
            callbacks.onItemFailed?.(code, lookupFailureMessage(result.error));
            continue;
          }
          progressPhase.value = "创建物品";
          callbacks.onItemStarted?.(code);
          const createResult = await createOne(
            code,
            result.candidate,
            options.value,
            controller.signal,
          );
          progressDone.value += 1;
          if ("cancelled" in createResult) return 0;
          if (createResult.ok) {
            createdCount += 1;
            callbacks.onItemCreated?.(code, createResult.item, createResult.created);
          } else {
            callbacks.onItemFailed?.(code, createResult.reason);
          }
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
    candidate: LcscItemLookupResponse,
    batchOptions: BatchLcscCreationOptions,
    signal: AbortSignal,
  ): Promise<BatchLcscItemResult> {
    const template =
      templates.value.find((candidate) => candidate.id === batchOptions.templateId) ?? null;
    const draft = emptyItemDraft();
    draft.unit = batchOptions.unit;
    draft.categoryId = batchOptions.categoryId;

    applyLcscLookupToDraft(draft, candidate, template);

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
      return { ok: true, item: created, created: true };
    } catch (error) {
      if (isAbort(error, signal)) return cancelledResult();
      if (error instanceof ApiError && error.code === "sku_taken") {
        // 并发或既有数据已占用该编号：视为已存在，直接匹配。
        await cleanupDraft();
        const existing = await findItemOption(code, null, signal).catch(() => null);
        if (existing) return { ok: true, item: existing, created: false };
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
    progressPhase,
    loadMetadata,
    defaultOptions,
    run,
    cancel,
  };
}

function lookupFailureMessage(error: LcscBatchLookupError | null): string {
  switch (error) {
    case "invalid_product_code":
      return "立创资料查询失败：客编格式无效";
    case "product_not_found":
      return "立创资料查询失败：未查询到该立创商品";
    case "timeout":
      return "立创资料查询失败：查询超时";
    case "busy":
      return "立创资料查询失败：查询繁忙";
    case "invalid_response":
      return "立创资料查询失败：立创返回了无法识别的数据";
    case "failed":
    default:
      return "立创资料查询失败，请重试";
  }
}

function normalizeProductCode(code: string): string {
  return code.trim().toUpperCase();
}

function isAbort(error: unknown, signal: AbortSignal): boolean {
  return signal.aborted || (error instanceof DOMException && error.name === "AbortError");
}

function cancelledResult(): BatchLcscItemResult {
  return { ok: false, cancelled: true, reason: "" };
}
