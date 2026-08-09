<!--
  本组件拥有第三方 ERP 备份导入 Dialog 的编排：选择 .xlsx 备份、解析预览（库位/物品/跳过）、
  真 C 码物品匹配与勾选批量创建、缺失库位新建，确认后回传期初库存行由入库装配写入草稿。
  它不写入草稿，也不拥有备份解析规则（见 erp/backupImport.ts）。
-->
<template>
  <ModalDialog
    :open="open"
    :title="$t('stockDraft.importErpBackupTitle')"
    :description="$t('stockDraft.importErpBackupDescription')"
    workspace
    :busy="importing || preparing || matching"
    @close="requestClose"
  >
    <div class="erp-backup-import">
      <div class="erp-backup-import__file-row">
        <label class="secondary-button erp-backup-import__file">
          {{ fileName ? $t('stockDraft.reselectFile') : $t('stockDraft.selectBackupFile') }}
          <input
            type="file"
            accept=".xlsx"
            :disabled="busy || preparing || matching"
            @change="handleFileChange"
          />
        </label>
        <span v-if="fileName" class="erp-backup-import__file-name" :title="fileName">
          {{ fileName }}
        </span>
      </div>

      <p v-if="parseError" class="erp-backup-import__error" role="alert">{{ parseError }}</p>
      <p v-else-if="parsing" class="erp-backup-import__hint" role="status">
        {{ $t('stockDraft.parsingBackup') }}
      </p>
      <p v-else-if="preparing" class="erp-backup-import__hint" role="status" aria-live="polite">
        {{ preparationMessage }}
      </p>
      <p v-if="matching" class="erp-backup-import__hint" role="status" aria-live="polite">
        {{ $t('stockDraft.matchingItemsProgress', { done: matchCompleted, total: matchTotal }) }}
      </p>
      <div v-if="preparationError" class="erp-backup-import__error" role="alert">
        {{ preparationError }}
        <button class="text-button" type="button" @click="retryPreparation">
          {{ $t('stockDraft.retryCheck') }}
        </button>
      </div>
      <div v-if="importError" class="erp-backup-import__error" role="alert">
        {{ importError }}
        <button class="text-button" type="button" @click="retryImport">
          {{ $t('stockDraft.retryImport') }}
        </button>
      </div>

      <template v-else-if="parsed">
        <p class="erp-backup-import__summary" role="status">
          {{
            $t('stockDraft.backupSummary', {
              locations: locations.length,
              new: newLocationCount,
              components: componentRows.length,
              matched: matchedComponentCount,
              creatable: creatableCount,
              items: parsed.items.length,
            })
          }}
        </p>

        <div v-if="duplicateExists" class="erp-backup-import__warn" role="alert">
          {{ $t('stockDraft.duplicateImportWarning', { fileName }) }}
        </div>

        <div v-if="parsed.skippedManual.length > 0" class="erp-backup-import__notice" role="status">
          <strong>{{ $t('stockDraft.skippedManualTitle', { n: parsed.skippedManual.length }) }}</strong>
          <span>{{ $t('stockDraft.skippedManualBody') }}</span>
        </div>

        <div v-overlay-scrollbar class="erp-backup-import__table-wrap">
          <table class="erp-backup-import__table">
            <colgroup>
              <col class="erp-backup-import__col--select" />
              <col class="erp-backup-import__col--name" />
              <col class="erp-backup-import__col--code" />
              <col class="erp-backup-import__col--quantity" />
              <col class="erp-backup-import__col--status" />
            </colgroup>
            <thead>
              <tr>
                <th scope="col" class="erp-backup-import__select-col">
                  <input
                    v-if="canCreateItem && creatableCount > 0"
                    type="checkbox"
                    class="erp-backup-import__select"
                    :checked="allCreatableSelected"
                    :indeterminate.prop="someCreatableSelected"
                    :disabled="busy"
                    :aria-label="$t('stockDraft.selectAllCreatable')"
                    @change="toggleSelectAllCreatable(($event.target as HTMLInputElement).checked)"
                  />
                </th>
                <th scope="col">{{ $t('stockDraft.component') }}</th>
                <th scope="col">{{ $t('stockDraft.code') }}</th>
                <th scope="col">{{ $t('stockDraft.stock') }}</th>
                <th scope="col">{{ $t('common.status') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in componentRows" :key="row.component.id">
                <td class="erp-backup-import__select-col">
                  <input
                    v-if="canCreateItem && isCreatable(row)"
                    v-model="row.selected"
                    type="checkbox"
                    class="erp-backup-import__select"
                    :disabled="busy"
                    :aria-label="$t('stockDraft.selectComponent', { code: row.component.partNumber })"
                  />
                </td>
                <td class="erp-backup-import__name" :title="row.component.name ?? ''">
                  {{ row.component.name ?? "—" }}
                </td>
                <td>{{ row.component.partNumber }}</td>
                <td>{{ row.totalQuantity }}</td>
                <td class="erp-backup-import__status-cell">
                  <template v-if="row.status === 'matched'">
                    <span class="erp-backup-import__status--ok" :title="row.item?.name"
                      >{{ $t('stockDraft.inStock') }}</span
                    >
                  </template>
                  <template v-else-if="row.status === 'created'">
                    <span class="erp-backup-import__status--ok">{{ $t('stockDraft.created') }}</span>
                  </template>
                  <template v-else-if="row.status === 'matching'"
                    >{{ $t('stockDraft.matching') }}</template
                  >
                  <template v-else-if="row.status === 'lookup'"
                    >{{ $t('stockDraft.lookupLcscData') }}</template
                  >
                  <template v-else-if="row.status === 'creating'"
                    >{{ $t('stockDraft.creating') }}</template
                  >
                  <template v-else-if="row.status === 'missing'"
                    >{{ $t('stockDraft.toBeCreated') }}</template
                  >
                  <template v-else-if="row.status === 'failed'">
                    <span :title="row.reason">{{ row.reason }}</span>
                  </template>
                  <template v-else-if="row.status === 'create-failed'">
                    <span :title="row.reason">{{ row.reason }}</span>
                  </template>
                  <span v-else :title="row.reason">{{ row.reason }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>

      <p v-else class="erp-backup-import__hint">
        {{ $t('stockDraft.backupHint') }}
      </p>
    </div>

    <template #actions>
      <button
        v-if="parsed && canCreateItem && (creatableCount > 0 || batch.running.value)"
        class="secondary-button erp-backup-import__batch-action"
        :class="{
          'erp-backup-import__action--pending': batch.metadataLoading.value || batch.running.value,
        }"
        type="button"
        :aria-busy="batch.metadataLoading.value || batch.running.value"
        :disabled="batch.metadataLoading.value || busy || selectedCreatableCount === 0"
        @click="openBatchCreate"
      >
        {{
          batch.running.value
            ? `${batch.progressLabel.value}…`
            : batch.metadataLoading.value
              ? $t('stockDraft.preparing')
              : $t('stockDraft.createSelectedItems', { n: selectedCreatableCount })
        }}
      </button>
      <button
        class="secondary-button"
        type="button"
        :disabled="importing || preparing || matching"
        @click="requestClose"
      >
        {{ $t('common.cancel') }}
      </button>
      <button
        v-if="parsed"
        class="primary-button erp-backup-import__import-action"
        :class="{ 'erp-backup-import__action--pending': importing }"
        type="button"
        :aria-busy="importing"
        :disabled="!canImport || busy"
        @click="confirmImport"
      >
        {{
          importing
            ? `${importProgressMessage}…`
            : $t('stockDraft.importStockLines', { n: importableCount })
        }}
      </button>
    </template>
  </ModalDialog>

  <BatchLcscCreateOptionsDialog
    :open="batchOptionsOpen"
    :count="selectedCreatableCount"
    :templates="batch.templates.value"
    :categories="batch.categories.value"
    :metadata-error="batch.metadataError.value"
    :initial-options="batch.defaultOptions()"
    @close="batchOptionsOpen = false"
    @confirm="startBatchCreate"
  />

  <ModalDialog
    :open="closeConfirmOpen"
    :title="$t('stockDraft.stopBatchCreateTitle')"
    :description="$t('stockDraft.stopBatchCreateDescription')"
    compact
    nested
    @close="closeConfirmOpen = false"
  >
    <template #actions>
      <button class="secondary-button" type="button" @click="closeConfirmOpen = false">
        {{ $t('stockDraft.continueCreating') }}
      </button>
      <button class="danger-button" type="button" @click="confirmCloseWhileCreating">
        {{ $t('stockDraft.stopAndClose') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script lang="ts">
import type { ItemOptionResponse } from "../../api/items";

/** 导入确认后回传的一条期初库存行；由入库装配写入草稿。 */
export interface ErpBackupImportRow {
  item: ItemOptionResponse;
  quantity: number;
  /** 目标库位；备份库位未匹配/未新建时为 null，交由草稿库位预填。 */
  locationId: number | null;
}

export interface ErpBackupImportPayload {
  fileName: string;
  appVersion: string | null;
  skippedManualCount: number;
  rows: ErpBackupImportRow[];
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { lookupItemOptions } from "../../api/items";
import { listInboundOrders } from "../../api/inboundOrders";
import { createLocation, listLocationGroupTree, listLocations } from "../../api/locations";
import { notice } from "../../notices/notice";
import { parseErpBackupFile } from "../../erp/backupImportFile";
import type { ErpBackupComponent, ErpBackupParseResult } from "../../erp/backupImport";
import { translateMessageOrNull } from "../../i18n";
import BatchLcscCreateOptionsDialog from "../items/BatchLcscCreateOptionsDialog.vue";
import {
  useBatchLcscItemCreation,
  type BatchLcscCreationOptions,
} from "../items/useBatchLcscItemCreation";
import ModalDialog from "../ModalDialog.vue";

type ComponentStatus =
  | "matching"
  | "matched"
  | "created"
  | "missing"
  | "failed"
  | "lookup"
  | "creating"
  | "create-failed";

interface ComponentRow {
  component: ErpBackupComponent;
  totalQuantity: number;
  status: ComponentStatus;
  reason: string;
  item: ItemOptionResponse | null;
  selected: boolean;
}

type ParsedBackup = Extract<ErpBackupParseResult, { ok: true }>;

const props = defineProps<{
  open: boolean;
  canCreateItem: boolean;
}>();

const emit = defineEmits<{
  close: [];
  import: [payload: ErpBackupImportPayload];
}>();

const fileName = ref("");
const parsing = ref(false);
const parseError = ref("");
const parsed = ref<ParsedBackup | null>(null);
const componentRows = ref<ComponentRow[]>([]);
const importing = ref(false);
const importProgressMessage = ref("");
const importProgressDone = ref(0);
const importProgressTotal = ref(0);
const importError = ref("");
const preparing = ref(false);
const preparationMessage = ref("");
const preparationError = ref("");
const matchTotal = ref(0);
const matchCompleted = ref(0);
const fileRunId = ref(0);
const batchOptionsOpen = ref(false);
const closeConfirmOpen = ref(false);
const batch = useBatchLcscItemCreation();
let matchAbortController: AbortController | null = null;

const { t } = useI18n();

/** 备份来源前缀写入单据来源，重复检测搜索按同一前缀匹配；属存储数据，不随 UI 语言变化。 */
const backupSourcePrefix = "\u5907\u4efd\u5bfc\u5165"; // 备份导入

const busy = computed(() => batch.running.value || importing.value);
const locations = computed(() => parsed.value?.locations ?? []);
const matchedComponentCount = computed(
  () =>
    componentRows.value.filter((row) => row.status === "matched" || row.status === "created")
      .length,
);
const creatableRows = computed(() => componentRows.value.filter((row) => isCreatable(row)));
const creatableCount = computed(() => creatableRows.value.length);
const selectedCreatableCount = computed(
  () => creatableRows.value.filter((row) => row.selected).length,
);
const allCreatableSelected = computed(
  () => creatableRows.value.length > 0 && creatableRows.value.every((row) => row.selected),
);
const someCreatableSelected = computed(
  () => creatableRows.value.some((row) => row.selected) && !allCreatableSelected.value,
);
/** 匹配中或待创建物品尚未就绪的器件仍在处理，导入前应先解决。 */
const matching = computed(() => componentRows.value.some((row) => row.status === "matching"));
/** 已匹配器件对应的库存行数（未匹配器件的库存行会被导入跳过）。 */
const importableCount = computed(() => {
  if (!parsed.value) return 0;
  const matchedParts = new Set(
    componentRows.value
      .filter((row) => row.status === "matched" || row.status === "created")
      .map((row) => row.component.partNumber),
  );
  return parsed.value.items.filter((item) => matchedParts.has(item.component.partNumber)).length;
});
const canImport = computed(
  () => !matching.value && !preparing.value && !preparationError.value && importableCount.value > 0,
);

const newLocationCount = ref(0);
const duplicateExists = ref(false);

function isCreatable(row: ComponentRow): boolean {
  return row.status === "missing" || row.status === "create-failed";
}
function toggleSelectAllCreatable(checked: boolean): void {
  for (const row of creatableRows.value) row.selected = checked;
}

watch(
  () => props.open,
  (open) => {
    if (open) resetState();
    else {
      matchAbortController?.abort();
      batch.cancel();
    }
  },
);

onBeforeUnmount(() => matchAbortController?.abort());

function resetState(): void {
  fileRunId.value += 1;
  matchAbortController?.abort();
  matchAbortController = null;
  batch.cancel();
  fileName.value = "";
  parsing.value = false;
  parseError.value = "";
  parsed.value = null;
  componentRows.value = [];
  importing.value = false;
  importProgressMessage.value = "";
  importProgressDone.value = 0;
  importProgressTotal.value = 0;
  importError.value = "";
  preparing.value = false;
  preparationMessage.value = "";
  preparationError.value = "";
  matchTotal.value = 0;
  matchCompleted.value = 0;
  batchOptionsOpen.value = false;
  closeConfirmOpen.value = false;
  newLocationCount.value = 0;
  duplicateExists.value = false;
}

async function handleFileChange(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  resetState();
  const runId = fileRunId.value;
  fileName.value = file.name;
  parsing.value = true;
  const result = await parseErpBackupFile(file);
  if (runId !== fileRunId.value) return;
  parsing.value = false;
  if (!result.ok) {
    // 解析层返回消息键，这里翻译后展示。
    parseError.value = translateMessageOrNull(result.error) ?? result.error;
    return;
  }
  parsed.value = result;
  // 器件按 C 号去重（同器件跨多库位），库存总量累加用于展示。
  const byPart = new Map<string, ComponentRow>();
  for (const item of result.items) {
    const existing = byPart.get(item.component.partNumber);
    if (existing) {
      existing.totalQuantity += item.quantity;
    } else {
      byPart.set(item.component.partNumber, {
        component: item.component,
        totalQuantity: item.quantity,
        status: "matching",
        reason: "",
        item: null,
        selected: true,
      });
    }
  }
  componentRows.value = [...byPart.values()];
  await prepareImport(runId);
}

async function prepareImport(runId: number): Promise<void> {
  if (!parsed.value || runId !== fileRunId.value) return;
  preparing.value = true;
  preparationError.value = "";
  preparationMessage.value = t("stockDraft.checkingDuplicateAndLocations");
  const results = await Promise.allSettled([
    matchComponents(runId),
    computeNewLocationCount(),
    checkDuplicate(),
  ]);
  if (runId !== fileRunId.value) return;
  const failures = results.filter((result) => result.status === "rejected");
  preparing.value = false;
  if (failures.length > 0) {
    preparationError.value = t("stockDraft.preparationFailed");
  }
}

function retryPreparation(): void {
  if (preparing.value || !parsed.value) return;
  for (const row of componentRows.value) {
    if (row.status === "failed") {
      row.status = "matching";
      row.reason = "";
    }
  }
  void prepareImport(fileRunId.value);
}

/** best-effort 重复导入检测：已存在以本文件名为来源的入库单则提示（不阻止）。 */
async function checkDuplicate(): Promise<void> {
  const page = await listInboundOrders({
    page: 1,
    page_size: 1,
    search: `${backupSourcePrefix} ${fileName.value}`,
  });
  duplicateExists.value = page.total > 0;
}

/** 按 C 号批量精确匹配库内物品，响应完成后一次性提交全部器件状态。 */
async function matchComponents(runId: number): Promise<void> {
  matchAbortController?.abort();
  const controller = new AbortController();
  matchAbortController = controller;
  const codes = [
    ...new Set(
      componentRows.value
        .filter((row) => row.status === "matching")
        .map((row) => row.component.partNumber),
    ),
  ];
  matchTotal.value = codes.length;
  matchCompleted.value = 0;
  if (codes.length === 0) return;
  try {
    const response = await lookupItemOptions(codes, controller.signal);
    if (controller.signal.aborted || runId !== fileRunId.value) return;
    const resultByCode = new Map(
      response.results.map((result) => [result.product_code.trim().toUpperCase(), result]),
    );
    componentRows.value = componentRows.value.map((row) => {
      if (row.status !== "matching") return row;
      const result = resultByCode.get(row.component.partNumber.trim().toUpperCase());
      if (result?.item) return { ...row, item: result.item, status: "matched", reason: "" };
      return {
        ...row,
        status: "missing",
        reason: result?.error === "not_found" ? t("stockDraft.codeNotFoundInStock") : "",
      };
    });
    matchCompleted.value = codes.length;
  } catch (error) {
    if (controller.signal.aborted || runId !== fileRunId.value) return;
    componentRows.value = componentRows.value.map((row) =>
      row.status === "matching"
        ? { ...row, status: "failed", reason: t("stockDraft.lookupFailedRetryCheck") }
        : row,
    );
    throw error;
  }
}

/** 预览"待新建库位"数：备份库位按 name===code 与现有库位比对。 */
async function computeNewLocationCount(): Promise<void> {
  if (!parsed.value) return;
  const existing = await listLocations();
  const names = new Set(existing.map((location) => location.name));
  newLocationCount.value = parsed.value.locations.filter(
    (location) => !names.has(location.code),
  ).length;
}

async function openBatchCreate(): Promise<void> {
  if (busy.value) return;
  await batch.loadMetadata();
  batchOptionsOpen.value = true;
}

async function startBatchCreate(options: BatchLcscCreationOptions): Promise<void> {
  batchOptionsOpen.value = false;
  const codes = creatableRows.value
    .filter((row) => row.selected)
    .map((row) => row.component.partNumber);
  if (codes.length === 0) return;
  await batch.run(codes, options, {
    onItemLookupStarted: (code) => setRowStatus(code, "lookup"),
    onItemStarted: (code) => setRowStatus(code, "creating"),
    onItemCreated: (code, item, created) => {
      const row = rowByCode(code);
      if (row) {
        row.item = item;
        row.status = created ? "created" : "matched";
        row.reason = "";
      }
    },
    onItemFailed: (code, reason) => setRowStatus(code, "create-failed", reason),
  });
  for (const row of componentRows.value) {
    if (row.status === "creating" || row.status === "lookup") row.status = "missing";
  }
}

function rowByCode(code: string): ComponentRow | undefined {
  return componentRows.value.find((row) => row.component.partNumber === code);
}
function setRowStatus(code: string, status: ComponentStatus, reason = ""): void {
  const row = rowByCode(code);
  if (row) {
    row.status = status;
    row.reason = reason;
  }
}

function requestClose(): void {
  if (batch.running.value) {
    closeConfirmOpen.value = true;
    return;
  }
  emit("close");
}

function confirmCloseWhileCreating(): void {
  closeConfirmOpen.value = false;
  batch.cancel();
  emit("close");
}

/** 落地库位（匹配现有/新建）→ 按已匹配器件组装期初库存行 → 回传装配。 */
async function confirmImport(): Promise<void> {
  if (!parsed.value || !canImport.value || busy.value) return;
  importing.value = true;
  importError.value = "";
  importProgressMessage.value = t("stockDraft.preparingLocations");
  try {
    const itemByPart = new Map(
      componentRows.value
        .filter((row) => (row.status === "matched" || row.status === "created") && row.item)
        .map((row) => [row.component.partNumber, row.item as ItemOptionResponse]),
    );
    const locationIdByCode = await resolveLocations();
    if (!locationIdByCode) return;

    importProgressMessage.value = t("stockDraft.assemblingStockLines", {
      n: parsed.value.items.length,
    });

    const rows = parsed.value.items
      .map((item) => {
        const target = itemByPart.get(item.component.partNumber);
        if (!target) return null;
        return {
          item: target,
          quantity: item.quantity,
          locationId: item.locationCode ? (locationIdByCode.get(item.locationCode) ?? null) : null,
        };
      })
      .filter((row): row is ErpBackupImportRow => row !== null);

    emit("import", {
      fileName: fileName.value,
      appVersion: parsed.value.appVersion,
      skippedManualCount: parsed.value.skippedManual.length,
      rows,
    });
  } finally {
    importing.value = false;
  }
}

function retryImport(): void {
  if (importing.value || !canImport.value) return;
  void confirmImport();
}

/** 备份库位按 name===code 匹配现有；缺失的在首个根分组（示例库区）下串行新建。 */
async function resolveLocations(): Promise<Map<string, number> | null> {
  if (!parsed.value) return null;
  importProgressTotal.value = parsed.value.locations.length;
  importProgressDone.value = 0;
  importProgressMessage.value = t("stockDraft.queryingLocations");
  try {
    const existing = await listLocations();
    const idByName = new Map(existing.map((location) => [location.name, location.id]));
    const result = new Map<string, number>();
    const missing = parsed.value.locations.filter((location) => !idByName.has(location.code));

    let groupId: number | null = null;
    if (missing.length > 0) {
      importProgressMessage.value = t("stockDraft.queryingLocationGroups");
      const tree = await listLocationGroupTree();
      groupId = tree[0]?.id ?? null;
      if (groupId === null) {
        importError.value = t("stockDraft.noLocationGroup");
        notice.error(t("stockDraft.cannotImport"), { detail: importError.value });
        return null;
      }
    }
    for (const location of parsed.value.locations) {
      importProgressMessage.value = t("stockDraft.preparingLocationProgress", {
        done: importProgressDone.value + 1,
        total: importProgressTotal.value,
      });
      const existingId = idByName.get(location.code);
      if (existingId !== undefined) {
        result.set(location.code, existingId);
        importProgressDone.value += 1;
        continue;
      }
      const created = await createLocation({ group_id: groupId as number, name: location.code });
      result.set(location.code, created.id);
      importProgressDone.value += 1;
    }
    return result;
  } catch (error) {
    importError.value =
      error instanceof Error
        ? t("stockDraft.locationPrepareFailedDetail", { message: error.message })
        : t("stockDraft.locationPrepareFailed");
    notice.error(t("stockDraft.locationPrepareFailedTitle"), { detail: importError.value });
    return null;
  }
}
</script>

<style scoped lang="scss">
.erp-backup-import {
  display: grid;
  gap: 12px;
  padding-top: 14px;
}

.erp-backup-import__file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.erp-backup-import__file {
  cursor: pointer;
  flex-shrink: 0;

  input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    clip-path: inset(50%);
    white-space: nowrap;
  }
}

.erp-backup-import__file-name {
  overflow: hidden;
  color: var(--color-muted);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.erp-backup-import__error {
  margin: 0;
  color: var(--color-danger);
  font-size: 13px;
}

.erp-backup-import__hint,
.erp-backup-import__summary {
  margin: 0;
  color: var(--color-muted);
  font-size: 13px;
  line-height: 1.6;
}

.erp-backup-import__warn {
  padding: 10px 12px;
  border: 1px solid var(--color-warn);
  border-radius: 8px;
  color: var(--color-warn);
  font-size: 13px;
  line-height: 1.6;
}

.erp-backup-import__notice {
  display: grid;
  gap: 3px;
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface-raised);
  font-size: 13px;
  line-height: 1.6;

  span {
    color: var(--color-muted);
  }
}

.erp-backup-import__table-wrap {
  overflow: auto;
  max-height: 300px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.erp-backup-import__table {
  width: 100%;
  min-width: 620px;
  table-layout: fixed;
  border-collapse: collapse;
  font-size: 13px;

  th,
  td {
    box-sizing: border-box;
    height: 34px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--color-border);
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  thead th {
    position: sticky;
    top: 0;
    height: 36px;
    background: var(--color-surface);
  }

  tbody tr:last-child td {
    border-bottom: none;
  }
}

.erp-backup-import__col--select {
  width: 40px;
}

.erp-backup-import__col--name {
  width: 28%;
}

.erp-backup-import__col--code {
  width: 22%;
}

.erp-backup-import__col--quantity {
  width: 80px;
}

.erp-backup-import__col--status {
  width: auto;
}

.erp-backup-import__select-col {
  width: 1%;
  padding-right: 4px;
  text-align: center;
}

.erp-backup-import__select {
  margin: 0;
  accent-color: var(--color-accent);
  cursor: pointer;
}

.erp-backup-import__name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.erp-backup-import__status-cell {
  overflow: hidden;
  text-overflow: ellipsis;
}

.erp-backup-import__status--ok {
  color: var(--color-success);
}

.erp-backup-import__batch-action {
  margin-right: auto;
}

.erp-backup-import__import-action:disabled {
  cursor: not-allowed;
}

.erp-backup-import__action--pending:disabled {
  cursor: wait;
}
</style>
