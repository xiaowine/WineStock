// 本文件拥有出库草稿的版本化 localStorage 保存与离开提示；不保存图片或调用业务 API。
import {
  onBeforeUnmount,
  onMounted,
  watch,
  type ComputedRef,
  type Ref,
} from "vue";
import type { OutboundDraftLine } from "../pages/outbound-draft/model";

const storageKey = "winestock.outbound-draft.v1";
interface PersistedDraft {
  version: 1;
  destination: string;
  notes: string;
  notesOpen: boolean;
  lines: OutboundDraftLine[];
}

/** 绑定出库草稿字段并提供恢复、删除和浏览器关闭保护。 */
export function useOutboundDraftPersistence(
  destination: Ref<string>,
  notes: Ref<string>,
  notesOpen: Ref<boolean>,
  lines: Ref<OutboundDraftLine[]>,
  hasDraft: ComputedRef<boolean>,
) {
  let suspended = true;
  watch([destination, notes, notesOpen, lines], save, { deep: true });
  onMounted(() => window.addEventListener("beforeunload", beforeUnload));
  onBeforeUnmount(() =>
    window.removeEventListener("beforeunload", beforeUnload),
  );
  function save() {
    if (suspended) return;
    if (!hasDraft.value) return remove();
    try {
      localStorage.setItem(
        storageKey,
        JSON.stringify({
          version: 1,
          destination: destination.value,
          notes: notes.value,
          notesOpen: notesOpen.value,
          lines: lines.value,
        } satisfies PersistedDraft),
      );
    } catch {
      /* 配额不足不阻断录入。 */
    }
  }
  function remove() {
    localStorage.removeItem(storageKey);
  }
  function restore(): boolean {
    try {
      const raw = localStorage.getItem(storageKey);
      if (!raw) return false;
      const draft = JSON.parse(raw) as PersistedDraft;
      if (draft.version !== 1 || !Array.isArray(draft.lines))
        throw new Error("invalid");
      destination.value = draft.destination || "";
      notes.value = draft.notes || "";
      notesOpen.value = Boolean(draft.notesOpen || notes.value);
      lines.value = draft.lines;
      return true;
    } catch {
      remove();
      return false;
    }
  }
  function beforeUnload(event: BeforeUnloadEvent) {
    if (!hasDraft.value) return;
    event.preventDefault();
    event.returnValue = "";
  }
  return {
    restoreDraft: restore,
    resumeDraftSaving: () => {
      suspended = false;
      save();
    },
    removePersistedDraft: remove,
  };
}
