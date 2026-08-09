<!--
  本组件拥有全局替代关系的主物品选择、共享编辑会话和未保存关闭保护。
  它复用单物品替代关系面板，不复制优先级与整体保存逻辑。
-->
<template>
  <ModalDialog
    :open="open"
    :title="target ? $t('substitutes.editorTitleEdit') : $t('substitutes.create')"
    :description="chosenTarget ? undefined : $t('substitutes.editorPickPrompt')"
    :busy="saving"
    workspace
    @close="requestClose"
  >
    <template v-if="chosenTarget" #context>
      <div class="substitute-editor-dialog__context">
        <div>
          <span>{{ $t('substitutes.currentPrimary') }}</span>
          <strong :title="chosenTarget.name">{{ chosenTarget.name }}</strong>
        </div>
        <div>
          <span>{{ $t('substitutes.skuLabel') }}</span>
          <strong :title="chosenTarget.sku">{{ chosenTarget.sku }}</strong>
        </div>
      </div>
    </template>

    <section v-if="!chosenTarget" class="substitute-target-picker" :aria-label="$t('substitutes.pickTargetLabel')">
      <SearchField
        v-model="searchInput"
        :label="$t('substitutes.searchTargetLabel')"
        name="substitute_primary_item_search"
        :placeholder="$t('substitutes.searchTargetPlaceholder')"
        autofocus
        @search="applySearch"
      />

      <div
        v-if="searchError"
        class="substitute-target-picker__state substitute-target-picker__state--error"
        role="alert"
      >
        <span>{{ searchError }}</span>
        <button class="secondary-button" type="button" @click="loadTargets">
          {{ $t('common.retry') }}
        </button>
      </div>
      <div v-else-if="searchLoading" class="substitute-target-picker__state" role="status">
        <span v-if="showSearchLoading">{{ $t('substitutes.searchingTarget') }}</span>
      </div>
      <div v-else-if="!activeSearch" class="substitute-target-picker__state">
        <strong>{{ $t('substitutes.searchTargetLabel') }}</strong>
        <span>{{ $t('substitutes.searchTargetHint') }}</span>
      </div>
      <div v-else-if="!targets.length" class="substitute-target-picker__state">
        <strong>{{ $t('substitutes.noTargetsTitle') }}</strong>
        <span>{{ $t('substitutes.noTargetsHint') }}</span>
      </div>
      <div
        v-else
        v-overlay-scrollbar
        class="substitute-target-picker__results"
        :aria-label="$t('substitutes.targetCandidatesLabel')"
      >
        <button v-for="item in targets" :key="item.id" type="button" @click="selectTarget(item)">
          <span
            ><strong :title="item.name">{{ item.name }}</strong
            ><small :title="item.sku">{{ $t('substitutes.skuLabel') }} {{ item.sku }}</small></span
          >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 5 7 7-7 7" /></svg>
        </button>
      </div>
    </section>

    <section v-else class="substitute-editor-dialog__workspace">
      <button
        v-if="!target"
        class="text-button substitute-editor-dialog__reselect"
        type="button"
        :disabled="saving"
        @click="requestReselect"
      >
        {{ $t('substitutes.reselectTarget') }}
      </button>
      <ItemSubstitutesPanel
        ref="substitutesPanel"
        :item-id="chosenTarget.id"
        :can-manage="canManage"
        :can-search-candidates="canSearchCandidates"
        :show-heading="false"
        :show-save-action="false"
        @dirty-change="handleDirtyChange"
        @saving-change="saving = $event"
        @save-error-change="saveError = $event"
        @saved="handleSaved"
      />
    </section>

    <template v-if="saveError" #notice>
      <p class="substitute-editor-dialog__save-error" role="alert">{{ saveError }}</p>
    </template>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="saving" @click="requestClose">
        {{ chosenTarget && canManage ? $t('common.cancel') : $t('common.close') }}
      </button>
      <button
        v-if="chosenTarget && canManage"
        class="primary-button substitute-editor-dialog__save"
        type="button"
        :disabled="saving || !dirty"
        @click="requestSave"
      >
        {{ saving ? $t('common.saving') : $t('substitutes.saveRelation') }}
      </button>
    </template>
  </ModalDialog>

  <ModalDialog
    :open="discardOpen"
    :title="$t('substitutes.discardTitle')"
    :description="$t('substitutes.discardDescription')"
    nested
    compact
    @close="cancelDiscard"
  >
    <p class="confirmation-copy">{{ $t('substitutes.discardHint') }}</p>
    <template #actions>
      <button class="secondary-button" type="button" @click="cancelDiscard">
        {{ $t('substitutes.continueEditing') }}
      </button>
      <button class="danger-button" type="button" @click="confirmDiscard">
        {{ $t('substitutes.discardChanges') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { listItemOptions, type ItemOptionResponse } from "../../api/items";
import type { SubstituteEditorTarget } from "../../pages/substitutes/model";
import { substituteErrorMessage } from "../../pages/substitutes/presentation";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import ModalDialog from "../ModalDialog.vue";
import SearchField from "../SearchField.vue";
import ItemSubstitutesPanel from "../items/ItemSubstitutesPanel.vue";
import "./SubstituteEditorDialog.scss";

const props = defineProps<{
  open: boolean;
  target: SubstituteEditorTarget | null;
  canManage: boolean;
  canSearchCandidates: boolean;
}>();

const emit = defineEmits<{
  close: [];
  saved: [target: SubstituteEditorTarget];
  "dirty-change": [dirty: boolean];
}>();

const chosenTarget = ref<SubstituteEditorTarget | null>(null);
const searchInput = ref("");
const activeSearch = ref("");
const targets = ref<ItemOptionResponse[]>([]);
const substitutesPanel = ref<{ requestSave: () => void } | null>(null);
const searchLoading = ref(false);
const searchError = ref("");
const saveError = ref("");
const dirty = ref(false);
const saving = ref(false);
const discardOpen = ref(false);
const pendingDiscardAction = ref<"close" | "reselect" | null>(null);
const showSearchLoading = useStablePendingIndicator(searchLoading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
let searchController: AbortController | null = null;

watch(
  () => props.open,
  (open) => {
    if (!open) {
      abortSearch();
      resetSession();
      return;
    }
    chosenTarget.value = props.target ? { ...props.target } : null;
  },
);

watch(
  () => props.target,
  (target) => {
    if (props.open) chosenTarget.value = target ? { ...target } : null;
  },
);

onBeforeUnmount(abortSearch);

function applySearch(value: string): void {
  activeSearch.value = value.trim();
  void loadTargets();
}

async function loadTargets(): Promise<void> {
  if (!activeSearch.value) {
    abortSearch();
    targets.value = [];
    searchError.value = "";
    return;
  }
  abortSearch();
  const controller = new AbortController();
  searchController = controller;
  searchLoading.value = true;
  searchError.value = "";
  try {
    const response = await listItemOptions(activeSearch.value, 1, 30, controller.signal);
    targets.value = response.items;
  } catch (error) {
    if (!(error instanceof DOMException && error.name === "AbortError"))
      searchError.value = substituteErrorMessage(error);
  } finally {
    if (searchController === controller) {
      searchController = null;
      searchLoading.value = false;
    }
  }
}

function selectTarget(item: ItemOptionResponse): void {
  chosenTarget.value = { id: item.id, name: item.name, sku: item.sku };
  abortSearch();
}

function handleDirtyChange(value: boolean): void {
  dirty.value = value;
  emit("dirty-change", value);
}

function handleSaved(): void {
  const current = chosenTarget.value;
  if (!current) return;
  dirty.value = false;
  saveError.value = "";
  emit("dirty-change", false);
  emit("saved", current);
  emit("close");
}

function requestSave(): void {
  substitutesPanel.value?.requestSave();
}

function requestClose(): void {
  if (saving.value) return;
  if (dirty.value) {
    pendingDiscardAction.value = "close";
    discardOpen.value = true;
    return;
  }
  emit("close");
}

function requestReselect(): void {
  if (dirty.value) {
    pendingDiscardAction.value = "reselect";
    discardOpen.value = true;
    return;
  }
  reselectTarget();
}

function cancelDiscard(): void {
  discardOpen.value = false;
  pendingDiscardAction.value = null;
}

function confirmDiscard(): void {
  const action = pendingDiscardAction.value;
  discardOpen.value = false;
  pendingDiscardAction.value = null;
  dirty.value = false;
  emit("dirty-change", false);
  if (action === "reselect") reselectTarget();
  else emit("close");
}

function reselectTarget(): void {
  chosenTarget.value = null;
  searchInput.value = "";
  activeSearch.value = "";
  targets.value = [];
  searchError.value = "";
  saveError.value = "";
}

function resetSession(): void {
  chosenTarget.value = null;
  searchInput.value = "";
  activeSearch.value = "";
  targets.value = [];
  searchError.value = "";
  saveError.value = "";
  dirty.value = false;
  saving.value = false;
  discardOpen.value = false;
  pendingDiscardAction.value = null;
  emit("dirty-change", false);
}

function abortSearch(): void {
  searchController?.abort();
  searchController = null;
  searchLoading.value = false;
}
</script>
