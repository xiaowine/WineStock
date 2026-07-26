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
    @close="emit('close')"
  >
    <div class="lcsc-order-import">
      <div class="lcsc-order-import__file-row">
        <label class="secondary-button lcsc-order-import__file">
          {{ fileName ? "重新选择文件" : "选择订单表格" }}
          <input type="file" accept=".xls,.xlsx" @change="handleFileChange" />
        </label>
        <span v-if="fileName" class="lcsc-order-import__file-name" :title="fileName">
          {{ fileName }}
        </span>
      </div>

      <p v-if="parseError" class="lcsc-order-import__error" role="alert">{{ parseError }}</p>
      <p v-else-if="parsing" class="lcsc-order-import__hint" role="status">正在解析表格…</p>

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

        <div class="lcsc-order-import__table-wrap">
          <table class="lcsc-order-import__table">
            <thead>
              <tr>
                <th scope="col">#</th>
                <th scope="col">商品编号</th>
                <th scope="col">数量</th>
                <th scope="col">单价</th>
                <th scope="col">状态</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in rows" :key="row.key" :class="rowClass(row)">
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
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button
        class="primary-button"
        type="button"
        :disabled="matchedCount === 0 || matching"
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
import { listItemOptions } from "../../api/items";
import { parseLcscOrderFile } from "../../lcsc/orderExportFile";
import ItemCreateDialog from "../items/ItemCreateDialog.vue";
import ModalDialog from "../ModalDialog.vue";

type PreviewStatus = "matching" | "matched" | "missing" | "excluded" | "failed";

interface PreviewRow {
  key: string;
  rowLabel: string;
  productCode: string | null;
  quantity: number | null;
  unitPrice: number | null;
  status: PreviewStatus;
  reason: string;
  item: ItemOptionResponse | null;
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
let matchAbortController: AbortController | null = null;

const matchedCount = computed(() => rows.value.filter((row) => row.status === "matched").length);
const missingCount = computed(() => rows.value.filter((row) => row.status === "missing").length);
const failedCount = computed(() => rows.value.filter((row) => row.status === "failed").length);
const excludedCount = computed(
  () => rows.value.filter((row) => row.status === "excluded" || row.status === "failed").length,
);
const matching = computed(() => rows.value.some((row) => row.status === "matching"));
const createTargetCode = computed(
  () => rows.value.find((row) => row.key === createTargetKey.value)?.productCode ?? "",
);

watch(
  () => props.open,
  (open) => {
    if (open) resetState();
    else matchAbortController?.abort();
  },
);

onBeforeUnmount(() => matchAbortController?.abort());

function resetState(): void {
  matchAbortController?.abort();
  matchAbortController = null;
  fileName.value = "";
  parsing.value = false;
  parseError.value = "";
  orderNo.value = null;
  rows.value = [];
  applySource.value = true;
  createTargetKey.value = null;
}

async function handleFileChange(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  resetState();
  fileName.value = file.name;
  parsing.value = true;
  const result = await parseLcscOrderFile(file);
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
    })),
  );
  rows.value = preview;
  await matchPendingRows();
}

function retryFailed(): void {
  for (const row of rows.value) {
    if (row.status === "failed") {
      row.status = "matching";
      row.reason = "";
    }
  }
  void matchPendingRows();
}

/** 按 C 号精确匹配库内物品；少量并发以兼顾速度与服务压力。 */
async function matchPendingRows(): Promise<void> {
  matchAbortController?.abort();
  const controller = new AbortController();
  matchAbortController = controller;
  const queue = rows.value.filter((row) => row.status === "matching");
  const workers = Array.from({ length: Math.min(4, queue.length) }, async () => {
    for (let row = queue.shift(); row; row = queue.shift()) {
      await matchRow(row, controller.signal);
    }
  });
  await Promise.all(workers);
}

async function matchRow(row: PreviewRow, signal: AbortSignal): Promise<void> {
  const code = row.productCode;
  if (!code) return;
  try {
    const response = await listItemOptions(code, 1, 20, signal);
    if (signal.aborted) return;
    const item =
      response.items.find((candidate) => candidate.sku.trim().toUpperCase() === code) ?? null;
    if (item) {
      row.item = item;
      row.status = "matched";
    } else {
      row.status = "missing";
    }
  } catch {
    if (signal.aborted) return;
    row.status = "failed";
    row.reason = "查询失败，可重试匹配";
  }
}

function openCreateFor(row: PreviewRow): void {
  createTargetKey.value = row.key;
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
  if (row.status === "missing") return "lcsc-order-import__row--missing";
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

.lcsc-order-import__source {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;

  input {
    margin: 0;
  }
}
</style>
