<!--
  本组件拥有立创订单导入 Dialog 的编排：选择 .xls 文件、解析预览、按 C 号匹配库内物品、
  未命中行的内嵌快速新建，以及来源勾选；确认后仅回传命中行数据。
  它不写入入库草稿，也不拥有表格解析规则（见 lcsc/orderExport.ts）。
-->
<template>
  <ModalDialog
    :open="open"
    title="导入立创订单"
    description="选择立创商城「订单详情」导出的表格，命中物品将按订购数量与单价加入入库明细。"
    workspace
    :busy="matching"
    @close="requestClose"
  >
    <div class="lcsc-order-import">
      <div class="lcsc-order-import__file-row">
        <label class="secondary-button lcsc-order-import__file">
          {{ fileName ? "重新选择文件" : "选择订单表格" }}
          <input
            type="file"
            accept=".xls,.xlsx"
            :disabled="matching || batchRunning"
            @change="handleFileChange"
          />
        </label>
        <span v-if="fileName" class="lcsc-order-import__file-name" :title="fileName">
          {{ fileName }}
        </span>
      </div>

      <p v-if="parseError" class="lcsc-order-import__error" role="alert">{{ parseError }}</p>
      <p v-else-if="parsing" class="lcsc-order-import__hint" role="status">正在解析表格…</p>
      <p v-else-if="matching" class="lcsc-order-import__hint" role="status" aria-live="polite">
        正在一次性匹配本地物品 {{ matchCompleted }}/{{ matchTotal }}…
      </p>
      <p v-if="matchError" class="lcsc-order-import__error" role="alert">
        {{ matchError }}
        <button class="text-button" type="button" @click="retryFailed">重试失败项</button>
      </p>

      <template v-else-if="rows.length > 0">
        <p class="lcsc-order-import__summary" role="status">
          <template v-if="orderNo">
            订单 <strong>{{ orderNo }}</strong
            >，
          </template>
          共 {{ rows.length }} 行：可导入 <strong>{{ matchedCount }}</strong> 行<template
            v-if="missingCount > 0"
            >，未命中 {{ missingCount }} 行</template
          ><template v-if="excludedCount > 0">，跳过 {{ excludedCount }} 行</template>。
          <button v-if="failedCount > 0" class="text-button" type="button" @click="retryFailed">
            重试匹配
          </button>
        </p>

        <div v-overlay-scrollbar class="lcsc-order-import__table-wrap">
          <table class="lcsc-order-import__table">
            <thead>
              <tr>
                <th scope="col" class="lcsc-order-import__select-col">
                  <input
                    v-if="canCreateItem && creatableCount > 0"
                    type="checkbox"
                    class="lcsc-order-import__select"
                    :checked="allCreatableSelected"
                    :indeterminate.prop="someCreatableSelected"
                    :disabled="batchRunning"
                    aria-label="全选待创建物品"
                    @change="toggleSelectAllCreatable(($event.target as HTMLInputElement).checked)"
                  />
                </th>
                <th scope="col">#</th>
                <th scope="col">商品编号</th>
                <th scope="col">数量</th>
                <th scope="col">单价</th>
                <th scope="col">状态</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in rows" :key="row.key" :class="rowClass(row)">
                <td class="lcsc-order-import__select-col">
                  <input
                    v-if="canCreateItem && isCreatable(row)"
                    v-model="row.selected"
                    type="checkbox"
                    class="lcsc-order-import__select"
                    :disabled="batchRunning"
                    :aria-label="`选择 ${row.productCode}`"
                  />
                </td>
                <td>{{ row.rowLabel }}</td>
                <td>{{ row.productCode ?? "—" }}</td>
                <td>{{ row.quantity ?? "—" }}</td>
                <td>{{ row.unitPrice !== null ? "¥" + row.unitPrice : "—" }}</td>
                <td>
                  <template v-if="row.status === 'matched'">
                    <span class="lcsc-order-import__status--ok" :title="row.item?.name">
                      {{ row.item?.name }}
                    </span>
                  </template>
                  <template v-else-if="row.status === 'matching'">匹配中…</template>
                  <template v-else-if="row.status === 'lookup'">查询立创资料…</template>
                  <template v-else-if="row.status === 'creating'">创建中…</template>
                  <template v-else-if="row.status === 'missing'">
                    <span>库中没有该编号</span>
                    <button
                      v-if="canCreateItem"
                      class="text-button"
                      type="button"
                      @click="openCreateFor(row)"
                    >
                      新建
                    </button>
                  </template>
                  <template v-else-if="row.status === 'create-failed'">
                    <span :title="row.reason">{{ row.reason }}</span>
                    <button
                      class="text-button"
                      type="button"
                      :disabled="batchRunning"
                      @click="retryBatchCreate(row)"
                    >
                      重试
                    </button>
                    <button
                      class="text-button"
                      type="button"
                      :disabled="batchRunning"
                      @click="openCreateFor(row)"
                    >
                      手动新建
                    </button>
                  </template>
                  <span v-else :title="row.reason">{{ row.reason }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <label v-if="orderNo && !sourceFilled" class="lcsc-order-import__source">
          <input v-model="applySource" type="checkbox" />
          <span>将本单来源填为“立创 {{ orderNo }}”</span>
        </label>
      </template>

      <p v-else class="lcsc-order-import__hint">
        表格中的收货、金额等其余信息不会导入；库位在导入后按明细逐条补齐。
      </p>
    </div>

    <template #actions>
      <button
        v-if="canCreateItem && (creatableCount > 0 || batchRunning)"
        class="secondary-button lcsc-order-import__batch-action"
        type="button"
        :disabled="batch.metadataLoading.value || batchRunning || selectedCreatableCount === 0"
        @click="openBatchCreate"
      >
        {{
          batchRunning
            ? `${batch.progressLabel.value}…`
            : batch.metadataLoading.value
              ? "准备中…"
              : `创建选中的 ${selectedCreatableCount} 个物品`
        }}
      </button>
      <button class="secondary-button" type="button" :disabled="matching" @click="requestClose">
        取消
      </button>
      <button
        class="primary-button"
        type="button"
        :disabled="matchedCount === 0 || matching || batchRunning"
        @click="confirmImport"
      >
        {{ matching ? "正在匹配…" : `加入 ${matchedCount} 条明细` }}
      </button>
    </template>
  </ModalDialog>

  <ItemCreateDialog
    :open="createTargetKey !== null"
    :initial-lcsc-code="createTargetCode"
    @close="createTargetKey = null"
    @created="handleItemCreated"
  />

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
    title="停止批量创建？"
    description="关闭后将停止尚未完成的创建；已经创建的物品会保留。"
    compact
    nested
    @close="closeConfirmOpen = false"
  >
    <template #actions>
      <button class="secondary-button" type="button" @click="closeConfirmOpen = false">
        继续创建
      </button>
      <button class="danger-button" type="button" @click="confirmCloseWhileCreating">
        停止并关闭
      </button>
    </template>
  </ModalDialog>
</template>

<script lang="ts">
import type { ItemOptionResponse } from "../../api/items";

/** 导入确认后回传的单行数据；由入库装配写入草稿。 */
export interface LcscOrderImportRow {
  item: ItemOptionResponse;
  quantity: number;
  unitPrice: number;
}

export interface LcscOrderImportPayload {
  orderNo: string | null;
  /** 用户是否勾选把来源填为「立创 SO…」。 */
  applySource: boolean;
  rows: LcscOrderImportRow[];
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { lookupItemOptions } from "../../api/items";
import { parseLcscOrderFile } from "../../lcsc/orderExportFile";
import BatchLcscCreateOptionsDialog from "../items/BatchLcscCreateOptionsDialog.vue";
import ItemCreateDialog from "../items/ItemCreateDialog.vue";
import {
  useBatchLcscItemCreation,
  type BatchLcscCreationOptions,
} from "../items/useBatchLcscItemCreation";
import ModalDialog from "../ModalDialog.vue";

type PreviewStatus =
  | "matching"
  | "matched"
  | "missing"
  | "excluded"
  | "failed"
  | "lookup"
  | "creating"
  | "create-failed";

interface PreviewRow {
  key: string;
  rowLabel: string;
  productCode: string | null;
  quantity: number | null;
  unitPrice: number | null;
  status: PreviewStatus;
  reason: string;
  item: ItemOptionResponse | null;
  /** 是否被勾选参与一键批量创建；仅对可创建行有意义，默认全选。 */
  selected: boolean;
}

const props = defineProps<{
  open: boolean;
  /** 草稿中已存在的物品 SKU（大写），命中即按“已在草稿”跳过。 */
  existingSkus: ReadonlySet<string>;
  canCreateItem: boolean;
  /** 来源已填写时不再提供来源勾选。 */
  sourceFilled: boolean;
}>();

const emit = defineEmits<{
  close: [];
  import: [payload: LcscOrderImportPayload];
}>();

const fileName = ref("");
const parsing = ref(false);
const parseError = ref("");
const orderNo = ref<string | null>(null);
const rows = ref<PreviewRow[]>([]);
const applySource = ref(true);
const createTargetKey = ref<string | null>(null);
const batchOptionsOpen = ref(false);
const closeConfirmOpen = ref(false);
const batch = useBatchLcscItemCreation();
const batchRunning = batch.running;
let matchAbortController: AbortController | null = null;
const matchTotal = ref(0);
const matchCompleted = ref(0);
const matchError = ref("");
const fileRunId = ref(0);

const matchedCount = computed(() => rows.value.filter((row) => row.status === "matched").length);
const missingCount = computed(
  () =>
    rows.value.filter((row) => row.status === "missing" || row.status === "create-failed").length,
);
const failedCount = computed(() => rows.value.filter((row) => row.status === "failed").length);
const excludedCount = computed(
  () => rows.value.filter((row) => row.status === "excluded" || row.status === "failed").length,
);
const matching = computed(() => rows.value.some((row) => row.status === "matching"));
/** 可参与一键创建的行：未命中或上次批量创建失败，且携带 C 号。 */
const creatableRows = computed(() =>
  rows.value.filter(
    (row) =>
      (row.status === "missing" || row.status === "create-failed") && row.productCode !== null,
  ),
);
const creatableCount = computed(() => creatableRows.value.length);
/** 勾选后将实际创建的行数；一键创建只作用于勾选项。 */
const selectedCreatableCount = computed(
  () => creatableRows.value.filter((row) => row.selected).length,
);
const allCreatableSelected = computed(
  () => creatableRows.value.length > 0 && creatableRows.value.every((row) => row.selected),
);
const someCreatableSelected = computed(
  () => creatableRows.value.some((row) => row.selected) && !allCreatableSelected.value,
);
function isCreatable(row: PreviewRow): boolean {
  return (row.status === "missing" || row.status === "create-failed") && row.productCode !== null;
}
function toggleSelectAllCreatable(checked: boolean): void {
  for (const row of creatableRows.value) row.selected = checked;
}
const createTargetCode = computed(
  () => rows.value.find((row) => row.key === createTargetKey.value)?.productCode ?? "",
);

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
  orderNo.value = null;
  rows.value = [];
  applySource.value = true;
  createTargetKey.value = null;
  batchOptionsOpen.value = false;
  closeConfirmOpen.value = false;
  matchTotal.value = 0;
  matchCompleted.value = 0;
  matchError.value = "";
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
  const result = await parseLcscOrderFile(file);
  if (runId !== fileRunId.value) return;
  parsing.value = false;
  if (!result.ok) {
    parseError.value = result.error;
    return;
  }
  orderNo.value = result.orderNo;
  const seenCodes = new Set<string>();
  const preview: PreviewRow[] = result.lines.map((line, index) => {
    const duplicated = seenCodes.has(line.productCode);
    seenCodes.add(line.productCode);
    const inDraft = props.existingSkus.has(line.productCode);
    return {
      key: `line-${index}`,
      rowLabel: line.rowLabel,
      productCode: line.productCode,
      quantity: line.quantity,
      unitPrice: line.unitPrice,
      status: duplicated || inDraft ? "excluded" : "matching",
      reason: duplicated ? "与前面的行重复" : inDraft ? "已在草稿中" : "",
      item: null,
      selected: true,
    };
  });
  preview.push(
    ...result.skipped.map((line, index): PreviewRow => ({
      key: `skipped-${index}`,
      rowLabel: line.rowLabel,
      productCode: line.productCode,
      quantity: null,
      unitPrice: null,
      status: "excluded",
      reason: line.reason,
      item: null,
      selected: false,
    })),
  );
  rows.value = preview;
  await matchPendingRows(runId);
}

function retryFailed(): void {
  if (matching.value || batchRunning.value) return;
  for (const row of rows.value) {
    if (row.status === "failed") {
      row.status = "matching";
      row.reason = "";
    }
  }
  matchError.value = "";
  void matchPendingRows(fileRunId.value);
}

/** 按 C 号批量精确匹配库内物品，响应完成后一次性提交全部行状态。 */
async function matchPendingRows(runId: number): Promise<void> {
  matchAbortController?.abort();
  const controller = new AbortController();
  matchAbortController = controller;
  const codes = [
    ...new Set(
      rows.value
        .filter((row) => row.status === "matching" && row.productCode)
        .map((row) => row.productCode as string),
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
    rows.value = rows.value.map((row) => {
      if (row.status !== "matching" || !row.productCode) return row;
      const result = resultByCode.get(row.productCode.trim().toUpperCase());
      if (result?.item) return { ...row, item: result.item, status: "matched", reason: "" };
      return {
        ...row,
        status: "missing",
        reason: result?.error === "not_found" ? "库中没有该编号" : "",
      };
    });
    matchCompleted.value = codes.length;
    matchError.value = "";
  } catch (error) {
    if (controller.signal.aborted || runId !== fileRunId.value) return;
    rows.value = rows.value.map((row) =>
      row.status === "matching"
        ? { ...row, status: "failed", reason: "查询失败，可重试匹配" }
        : row,
    );
    matchError.value =
      error instanceof Error ? `本地物品匹配失败：${error.message}` : "本地物品匹配失败，请重试。";
  }
}

function openCreateFor(row: PreviewRow): void {
  createTargetKey.value = row.key;
}

/** 打开批量创建选项对话框；模板与分类元数据首次点击时懒加载。 */
async function openBatchCreate(): Promise<void> {
  if (batchRunning.value) return;
  await batch.loadMetadata();
  batchOptionsOpen.value = true;
}

/** 确认选项后串行批量创建勾选的未匹配行；单项失败不阻塞后续。 */
async function startBatchCreate(options: BatchLcscCreationOptions): Promise<void> {
  batchOptionsOpen.value = false;
  const codes = creatableRows.value
    .filter((row) => row.selected)
    .map((row) => row.productCode as string);
  await runBatchCreate(codes, options);
}

/** 单行重试：沿用本次会话记住的批次选项，不再弹选项对话框。 */
async function retryBatchCreate(row: PreviewRow): Promise<void> {
  if (!row.productCode || batchRunning.value) return;
  await runBatchCreate([row.productCode], batch.defaultOptions());
}

async function runBatchCreate(codes: string[], options: BatchLcscCreationOptions): Promise<void> {
  if (codes.length === 0) return;
  await batch.run(codes, options, {
    onItemLookupStarted: (code) => {
      for (const row of rowsByCode(code)) {
        row.status = "lookup";
        row.reason = "";
      }
    },
    onItemStarted: (code) => {
      for (const row of rowsByCode(code)) {
        row.status = "creating";
        row.reason = "";
      }
    },
    onItemCreated: (code, item) => {
      for (const row of rowsByCode(code)) {
        row.item = item;
        row.status = "matched";
        row.reason = "";
      }
    },
    onItemFailed: (code, reason) => {
      for (const row of rowsByCode(code)) {
        row.status = "create-failed";
        row.reason = reason;
      }
    },
  });
  // 中止（关闭 Dialog 等）时把仍处于"创建中"的行放回未命中，避免状态卡死。
  for (const row of rows.value) {
    if (row.status === "creating" || row.status === "lookup") {
      row.status = "missing";
      row.reason = "";
    }
  }
}

function requestClose(): void {
  if (batchRunning.value) {
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

function rowsByCode(code: string): PreviewRow[] {
  return rows.value.filter(
    (row) =>
      row.productCode === code &&
      (row.status === "missing" ||
        row.status === "create-failed" ||
        row.status === "creating" ||
        row.status === "lookup"),
  );
}

function handleItemCreated(item: ItemOptionResponse): void {
  const row = rows.value.find((candidate) => candidate.key === createTargetKey.value);
  createTargetKey.value = null;
  if (!row) return;
  row.item = item;
  row.status = "matched";
  row.reason = "";
}

function confirmImport(): void {
  const imported = rows.value
    .filter((row) => row.status === "matched" && row.item)
    .map((row) => ({
      item: row.item as ItemOptionResponse,
      quantity: row.quantity ?? 1,
      unitPrice: row.unitPrice ?? 0,
    }));
  if (imported.length === 0) return;
  emit("import", {
    orderNo: orderNo.value,
    applySource: applySource.value && orderNo.value !== null && !props.sourceFilled,
    rows: imported,
  });
}

function rowClass(row: PreviewRow): string | undefined {
  if (row.status === "excluded" || row.status === "failed") return "lcsc-order-import__row--muted";
  if (row.status === "missing" || row.status === "create-failed")
    return "lcsc-order-import__row--missing";
  return undefined;
}
</script>

<style scoped lang="scss">
.lcsc-order-import {
  display: grid;
  gap: 12px;
  /* workspace 宽 Dialog 的 modal-body 顶部内边距为 0（预期由内容自理），这里补回与 modal-context 一致的 14px。 */
  padding-top: 14px;
}

.lcsc-order-import__file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.lcsc-order-import__file {
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

.lcsc-order-import__file-name {
  overflow: hidden;
  color: var(--color-muted);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lcsc-order-import__error {
  margin: 0;
  color: var(--color-danger);
  font-size: 13px;
}

.lcsc-order-import__hint,
.lcsc-order-import__summary {
  margin: 0;
  color: var(--color-muted);
  font-size: 13px;
  line-height: 1.6;
}

.lcsc-order-import__table-wrap {
  overflow: auto;
  max-height: 320px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

.lcsc-order-import__table {
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

.lcsc-order-import__select-col {
  width: 1%;
  padding-right: 4px;
  text-align: center;
}

.lcsc-order-import__select {
  margin: 0;
  accent-color: var(--color-accent);
  cursor: pointer;
}

.lcsc-order-import__row--muted td {
  color: var(--color-muted);
}

.lcsc-order-import__row--missing td {
  color: var(--color-warn);
}

.lcsc-order-import__status--ok {
  display: inline-block;
  overflow: hidden;
  max-width: 220px;
  vertical-align: bottom;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 批量创建是独立于"取消/加入明细"决策流的辅助操作，按项目惯例左置。 */
.lcsc-order-import__batch-action {
  margin-right: auto;
}

.lcsc-order-import__source {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;

  input {
    margin: 0;
    accent-color: var(--color-accent);
  }
}
</style>
