<!--
  本组件拥有第三方 ERP 备份导入 Dialog 的编排：选择 .xlsx 备份、解析预览（库位/物品/跳过）、
  真 C 码物品匹配与勾选批量创建、缺失库位新建，确认后回传期初库存行由入库装配写入草稿。
  它不写入草稿，也不拥有备份解析规则（见 erp/backupImport.ts）。
-->
<template>
  <ModalDialog
    :open="open"
    title="导入 ERP 备份"
    description="选择第三方「LCSC Android ERP」导出的 .xlsx 备份，按备份库存生成一张期初入库草稿。"
    workspace
    @close="emit('close')"
  >
    <div class="erp-backup-import">
      <div class="erp-backup-import__file-row">
        <label class="secondary-button erp-backup-import__file">
          {{ fileName ? "重新选择文件" : "选择备份文件" }}
          <input type="file" accept=".xlsx" @change="handleFileChange" />
        </label>
        <span v-if="fileName" class="erp-backup-import__file-name" :title="fileName">
          {{ fileName }}
        </span>
      </div>

      <p v-if="parseError" class="erp-backup-import__error" role="alert">{{ parseError }}</p>
      <p v-else-if="parsing" class="erp-backup-import__hint" role="status">正在解析备份…</p>

      <template v-else-if="parsed">
        <p class="erp-backup-import__summary" role="status">
          共 <strong>{{ locations.length }}</strong> 个库位（待新建
          {{ newLocationCount }} 个）、<strong>{{ componentRows.length }}</strong> 种器件（已匹配
          {{ matchedComponentCount }}、待创建 {{ creatableCount }}）、库存
          <strong>{{ parsed.items.length }}</strong> 条。
        </p>

        <div v-if="duplicateExists" class="erp-backup-import__warn" role="alert">
          已存在以「{{ fileName }}」为来源的入库单，该备份可能已导入过——继续会使库存数量翻倍。
        </div>

        <div v-if="parsed.skippedManual.length > 0" class="erp-backup-import__notice" role="status">
          <strong>已跳过 {{ parsed.skippedManual.length }} 项手工录入器件</strong>
          <span>
            编号以 C0
            开头的器件（C01、C02…）是原软件里手工创建的，不对应立创商品，无法在线获取资料，
            本次导入不包含。如需保留，请在导入完成后手工创建这些物品。
          </span>
        </div>

        <div class="erp-backup-import__table-wrap">
          <table class="erp-backup-import__table">
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
                    aria-label="全选待创建器件"
                    @change="toggleSelectAllCreatable(($event.target as HTMLInputElement).checked)"
                  />
                </th>
                <th scope="col">器件</th>
                <th scope="col">编号</th>
                <th scope="col">库存</th>
                <th scope="col">状态</th>
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
                    :aria-label="`选择 ${row.component.partNumber}`"
                  />
                </td>
                <td class="erp-backup-import__name" :title="row.component.name ?? ''">
                  {{ row.component.name ?? "—" }}
                </td>
                <td>{{ row.component.partNumber }}</td>
                <td>{{ row.totalQuantity }}</td>
                <td>
                  <template v-if="row.status === 'matched'">
                    <span class="erp-backup-import__status--ok" :title="row.item?.name"
                      >已在库</span
                    >
                  </template>
                  <template v-else-if="row.status === 'matching'">匹配中…</template>
                  <template v-else-if="row.status === 'creating'">创建中…</template>
                  <template v-else-if="row.status === 'missing'">待创建</template>
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
        备份里的图片不会解析，物品资料按 C 号在线补齐；C0 手工器件不导入。
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
            ? `正在创建 ${batch.progressLabel.value}…`
            : batch.metadataLoading.value
              ? "准备中…"
              : `创建选中的 ${selectedCreatableCount} 个物品`
        }}
      </button>
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button
        v-if="parsed"
        class="primary-button erp-backup-import__import-action"
        :class="{ 'erp-backup-import__action--pending': importing }"
        type="button"
        :aria-busy="importing"
        :disabled="!canImport || busy"
        @click="confirmImport"
      >
        {{ importing ? "正在导入…" : `导入 ${importableCount} 条库存` }}
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
import { listItemOptions } from "../../api/items";
import { listInboundOrders } from "../../api/inboundOrders";
import { createLocation, listLocationGroupTree, listLocations } from "../../api/locations";
import { notice } from "../../notices/notice";
import { parseErpBackupFile } from "../../erp/backupImportFile";
import type { ErpBackupComponent, ErpBackupParseResult } from "../../erp/backupImport";
import BatchLcscCreateOptionsDialog from "../items/BatchLcscCreateOptionsDialog.vue";
import {
  useBatchLcscItemCreation,
  type BatchLcscCreationOptions,
} from "../items/useBatchLcscItemCreation";
import ModalDialog from "../ModalDialog.vue";

type ComponentStatus = "matching" | "matched" | "missing" | "creating" | "create-failed";

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
const batchOptionsOpen = ref(false);
const batch = useBatchLcscItemCreation();
let matchAbortController: AbortController | null = null;

const busy = computed(() => batch.running.value || importing.value);
const locations = computed(() => parsed.value?.locations ?? []);
const matchedComponentCount = computed(
  () => componentRows.value.filter((row) => row.status === "matched").length,
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
      .filter((row) => row.status === "matched")
      .map((row) => row.component.partNumber),
  );
  return parsed.value.items.filter((item) => matchedParts.has(item.component.partNumber)).length;
});
const canImport = computed(() => !matching.value && importableCount.value > 0);

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
  matchAbortController?.abort();
  matchAbortController = null;
  batch.cancel();
  fileName.value = "";
  parsing.value = false;
  parseError.value = "";
  parsed.value = null;
  componentRows.value = [];
  importing.value = false;
  batchOptionsOpen.value = false;
  newLocationCount.value = 0;
  duplicateExists.value = false;
}

async function handleFileChange(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  resetState();
  fileName.value = file.name;
  parsing.value = true;
  const result = await parseErpBackupFile(file);
  parsing.value = false;
  if (!result.ok) {
    parseError.value = result.error;
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
  await Promise.all([matchComponents(), computeNewLocationCount(), checkDuplicate()]);
}

/** best-effort 重复导入检测：已存在以本文件名为来源的入库单则提示（不阻止）。 */
async function checkDuplicate(): Promise<void> {
  try {
    const page = await listInboundOrders({
      page: 1,
      page_size: 1,
      search: `备份导入 ${fileName.value}`,
    });
    duplicateExists.value = page.total > 0;
  } catch {
    duplicateExists.value = false;
  }
}

/** 按 C 号精确匹配库内物品；少量并发以兼顾速度与服务压力。 */
async function matchComponents(): Promise<void> {
  matchAbortController?.abort();
  const controller = new AbortController();
  matchAbortController = controller;
  const queue = componentRows.value.filter((row) => row.status === "matching");
  const workers = Array.from({ length: Math.min(4, queue.length) }, async () => {
    for (let row = queue.shift(); row; row = queue.shift()) {
      await matchOne(row, controller.signal);
    }
  });
  await Promise.all(workers);
}

async function matchOne(row: ComponentRow, signal: AbortSignal): Promise<void> {
  const code = row.component.partNumber;
  try {
    const response = await listItemOptions(code, 1, 20, signal);
    if (signal.aborted) return;
    const item = response.items.find((candidate) => candidate.sku.trim().toUpperCase() === code);
    if (item) {
      row.item = item;
      row.status = "matched";
    } else {
      row.status = "missing";
    }
  } catch {
    if (signal.aborted) return;
    row.status = "missing";
  }
}

/** 预览"待新建库位"数：备份库位按 name===code 与现有库位比对。 */
async function computeNewLocationCount(): Promise<void> {
  if (!parsed.value) return;
  try {
    const existing = await listLocations();
    const names = new Set(existing.map((location) => location.name));
    newLocationCount.value = parsed.value.locations.filter(
      (location) => !names.has(location.code),
    ).length;
  } catch {
    newLocationCount.value = 0;
  }
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
    onItemStarted: (code) => setRowStatus(code, "creating"),
    onItemCreated: (code, item) => {
      const row = rowByCode(code);
      if (row) {
        row.item = item;
        row.status = "matched";
        row.reason = "";
      }
    },
    onItemFailed: (code, reason) => setRowStatus(code, "create-failed", reason),
  });
  for (const row of componentRows.value) {
    if (row.status === "creating") row.status = "missing";
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

/** 落地库位（匹配现有/新建）→ 按已匹配器件组装期初库存行 → 回传装配。 */
async function confirmImport(): Promise<void> {
  if (!parsed.value || !canImport.value || busy.value) return;
  importing.value = true;
  try {
    const itemByPart = new Map(
      componentRows.value
        .filter((row) => row.status === "matched" && row.item)
        .map((row) => [row.component.partNumber, row.item as ItemOptionResponse]),
    );
    const locationIdByCode = await resolveLocations();
    if (!locationIdByCode) return;

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

/** 备份库位按 name===code 匹配现有；缺失的在首个根分组（示例库区）下串行新建。 */
async function resolveLocations(): Promise<Map<string, number> | null> {
  if (!parsed.value) return null;
  try {
    const existing = await listLocations();
    const idByName = new Map(existing.map((location) => [location.name, location.id]));
    const result = new Map<string, number>();
    const missing = parsed.value.locations.filter((location) => !idByName.has(location.code));

    let groupId: number | null = null;
    if (missing.length > 0) {
      const tree = await listLocationGroupTree();
      groupId = tree[0]?.id ?? null;
      if (groupId === null) {
        notice.error("无法导入", { detail: "没有可用的库位分组来新建库位。" });
        return null;
      }
    }
    for (const location of parsed.value.locations) {
      const existingId = idByName.get(location.code);
      if (existingId !== undefined) {
        result.set(location.code, existingId);
        continue;
      }
      const created = await createLocation({ group_id: groupId as number, name: location.code });
      result.set(location.code, created.id);
    }
    return result;
  } catch (error) {
    notice.error("库位准备失败", {
      detail: error instanceof Error ? error.message : "请稍后重试。",
    });
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
  border-collapse: collapse;
  font-size: 13px;

  th,
  td {
    padding: 6px 10px;
    border-bottom: 1px solid var(--color-border);
    text-align: left;
    white-space: nowrap;
  }

  thead th {
    position: sticky;
    top: 0;
    background: var(--color-surface);
  }

  tbody tr:last-child td {
    border-bottom: none;
  }
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
  max-width: 240px;
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
