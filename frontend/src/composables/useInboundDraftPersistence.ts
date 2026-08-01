// 本文件拥有入库草稿的 localStorage 序列化与恢复；它不提交业务 API 或处理页面离开确认。
import { watch, type ComputedRef, type Ref } from "vue";
import type { ItemOptionResponse } from "../api/items";
import { createLineId, type InboundDraftLine } from "../pages/inbound-draft/model";
import { clearInboundDraftImages } from "../storage/inboundDraftImageStore";

const storageKey = "winestock.inbound-draft.v6";
// 旧版本草稿包含入库模板字段与 IndexedDB 草稿图片，结构已失效，恢复时直接清除。
const obsoleteStorageKeys = [
  "winestock.inbound-draft.v5",
  "winestock.inbound-draft.v4",
  "winestock.inbound-draft.v3",
];

interface PersistedDraft {
  version: 6;
  source: string;
  notes: string;
  notesOpen: boolean;
  lines: Array<{
    lineId: string;
    item: ItemOptionResponse;
    quantity: number;
    unitPrice: number;
    locationId: number | null;
    batchNo: string;
    expiresAt: string;
  }>;
}

/** 绑定页面草稿引用；调用 restore 后再 resume，避免恢复过程中覆盖本地记录。 */
export function useInboundDraftPersistence(
  source: Ref<string>,
  notes: Ref<string>,
  notesOpen: Ref<boolean>,
  lines: Ref<InboundDraftLine[]>,
  hasDraft: ComputedRef<boolean>,
) {
  let suspended = true;
  watch([source, notes, notesOpen, lines], save, { deep: true });

  function resume(): void {
    suspended = false;
    save();
  }
  function remove(): void {
    localStorage.removeItem(storageKey);
  }

  function restore(): boolean {
    for (const key of obsoleteStorageKeys) localStorage.removeItem(key);
    void clearInboundDraftImages().catch(() => undefined);
    const raw = localStorage.getItem(storageKey);
    if (!raw) return false;
    try {
      const draft = JSON.parse(raw) as PersistedDraft;
      if (draft.version !== 6 || !Array.isArray(draft.lines)) throw new Error("invalid draft");
      source.value = typeof draft.source === "string" ? draft.source : "";
      notes.value = typeof draft.notes === "string" ? draft.notes : "";
      notesOpen.value = Boolean(draft.notesOpen || notes.value);
      lines.value = draft.lines.map((line) => ({
        lineId: line.lineId || createLineId(),
        item: line.item,
        quantity: line.quantity,
        unitPrice: line.unitPrice,
        locationId: line.locationId,
        batchNo: line.batchNo || "",
        expiresAt: line.expiresAt || "",
      }));
      return true;
    } catch {
      remove();
      return false;
    }
  }

  function save(): void {
    if (suspended) return;
    if (!hasDraft.value) {
      remove();
      return;
    }
    const draft: PersistedDraft = {
      version: 6,
      source: source.value,
      notes: notes.value,
      notesOpen: notesOpen.value,
      lines: lines.value.map((line) => ({
        lineId: line.lineId,
        item: line.item,
        quantity: line.quantity,
        unitPrice: line.unitPrice,
        locationId: line.locationId,
        batchNo: line.batchNo,
        expiresAt: line.expiresAt,
      })),
    };
    try {
      localStorage.setItem(storageKey, JSON.stringify(draft));
    } catch {
      /* 配额失败不阻断当前录入。 */
    }
  }

  return { restoreDraft: restore, resumeDraftSaving: resume, removePersistedDraft: remove };
}
