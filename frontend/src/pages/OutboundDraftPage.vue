<!-- 本页面拥有新建出库单的单物品串行配置工作台；不执行审批或库存扣减。 -->
<template>
  <section class="route-page outbound-draft-page">
    <header class="content-header outbound-draft-page__header">
      <div class="outbound-title">
        <h1>{{ $route.meta.title }}</h1>
      </div>
      <div v-if="lines.length" class="content-summary outbound-summary" aria-label="当前出库草稿摘要">
        <span><strong>{{ lines.length }}</strong> 条明细</span>
        <span>·</span>
        <span>出库数量 <strong>{{ quantitySummary }}</strong></span>
        <template v-if="costSummary.state === 'complete'">
          <span>·</span>
          <span>预计成本 <strong>¥{{ formatMoney(costSummary.amount ?? 0) }}</strong></span>
        </template>
        <template v-else-if="costSummary.state === 'loading'">
          <span>·</span><span>正在估算成本…</span>
        </template>
        <template v-else-if="costSummary.state === 'insufficient' || costSummary.state === 'failed'">
          <span>·</span><span>成本以实际出库为准</span>
        </template>
      </div>
      <div class="outbound-actions">
        <button
          class="text-button outbound-clear"
          type="button"
          :disabled="!hasDraft || submitting"
          @click="confirmMode = 'clear'"
        >
          清空草稿
        </button>
        <button
          v-if="lines.length"
          class="primary-button"
          type="button"
          :disabled="submitting"
          @click="review"
        >
          {{ submitting ? "正在提交…" : canDirectOutbound ? "直接出库" : "提交审核" }}
        </button>
      </div>
    </header>

    <section v-if="!canReadItems" class="outbound-blocked">
      <h2>无法读取可出库物品</h2>
      <p>
        当前账号具备创建出库单权限，但缺少物品与库存批次读取权限。请联系管理员授予“查看库存物品”权限后继续。
      </p>
      <button class="secondary-button" type="button" @click="router.back()">返回</button>
    </section>

    <section v-else class="outbound-workspace">
      <header class="outbound-workspace__header">
        <div>
          <h2>出库单信息与明细</h2>
          <p>添加后立即配置该物品；未完成明细会锁定下一次添加。</p>
        </div>
        <button
          class="primary-button"
          type="button"
          :disabled="!canAddItem"
          :title="canAddItem ? '选择物品并配置明细' : addItemDisabledReason"
          @click="openItemPicker"
        >
          添加物品
        </button>
      </header>

      <p v-if="!canAddItem && addItemDisabledReason" class="outbound-add-item-hint" role="status">
        {{ addItemDisabledReason }}
      </p>

      <section class="outbound-meta" aria-label="出库单基础信息">
        <label>
          <span>出库去向 *</span>
          <input
            ref="destinationInput"
            v-model="destination"
            data-outbound-destination
            name="outbound_destination"
            :class="{ error: validation && !destination.trim() }"
            maxlength="128"
            placeholder="客户 / 部门 / 项目"
          />
        </label>
        <button
          class="icon-button outbound-notes-toggle"
          :class="{ 'outbound-notes-toggle--filled': notes.trim() }"
          type="button"
          :title="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          :aria-label="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          :aria-expanded="notesOpen"
          @click="notesOpen = !notesOpen"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 4h14v12H9l-4 4V4Z" />
            <path d="M8 8h8M8 12h5" />
          </svg>
        </button>
        <label v-if="notesOpen" class="outbound-notes">
          <span>备注</span>
          <input
            v-model="notes"
            name="outbound_notes"
            maxlength="1024"
            placeholder="可选，记录出库说明"
          />
        </label>
      </section>

      <section class="outbound-lines" aria-label="出库明细复核">
        <div v-if="!lines.length" class="outbound-lines-empty">
          <strong>还没有出库明细</strong>
          <span>点击“添加物品”选择一项，完成对应出库明细。</span>
        </div>
        <article
          v-for="line in lines"
          :key="line.lineId"
          :class="{ invalid: validation && lineError(line) }"
        >
          <div class="outbound-line-identity">
            <AuthenticatedImage
              :file-id="line.item.image_file_id"
              :alt="`${line.item.name} 主图`"
              :size="38"
              previewable
            />
            <div>
              <strong :title="line.item.name">{{ line.item.name }}</strong>
              <small>{{ line.item.sku }} · {{ line.item.unit }}</small>
            </div>
          </div>
          <div class="outbound-line-summary">
            <span>数量：{{ quantityLabel(line) }}</span>
            <strong>{{ allocationPrimary(line) }}</strong>
            <span>{{ allocationSecondary(line) }}</span>
            <span class="outbound-line-summary__cost">{{ costEstimateLabel(line) }}</span>
          </div>
          <div class="outbound-line-actions">
            <button
              class="icon-button outbound-line-edit"
              type="button"
              :data-line-action="line.lineId"
              :aria-label="`${line.item.name}：${allocationSummary(line)}，打开出库明细编辑器`"
              title="编辑出库明细"
              @click="openAllocation(line)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z" />
                <path d="m15 7 2 2" />
              </svg>
            </button>
            <button
              class="icon-button outbound-line-remove"
              type="button"
              :aria-label="`移除 ${line.item.name}`"
              title="移除明细"
              @click="remove(line.lineId)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
              </svg>
            </button>
          </div>
        </article>
      </section>
    </section>

    <ItemSelectionDialog
      :open="itemPickerOpen"
      title="选择出库物品"
      description="选择一项后进入明细配置。"
      search-name="outbound_item_search"
      :items="items"
      :search-input="searchInput"
      :loading-items="loadingItems"
      :item-error="itemError"
      :items-exhausted="itemsExhausted"
      :selected-item-ids="selectedItemIds"
      :can-create-item="false"
      @close="closeItemPicker"
      @after-close="handleItemPickerAfterClose"
      @update:search-input="searchInput = $event"
      @search="applySearch"
      @reset-items="resetItems"
      @load-next-items="loadNextItems"
      @scroll-items="handleItemScroll"
      @list-element="setItemList"
      @select-item="handleItemSelected"
    />

    <ModalDialog
      :open="allocationLine !== null"
      title="配置出库明细"
      description="数量和扣减方式属于同一条明细；完成后才能继续添加下一项。"
      wide
      @close="closeAllocation"
      @after-close="handleAllocationAfterClose"
    >
      <template #context>
        <div v-if="allocationLine" class="outbound-allocation-context">
          <strong>{{ allocationLine.item.name }}</strong>
          <span>{{ allocationLine.item.sku }} · {{ allocationLine.item.unit }}</span>
        </div>
      </template>
      <template v-if="allocationLine">
        <section class="outbound-allocation-section">
          <header><strong>本次出库数量</strong><span>审批时仍会按实际库存重新校验。</span></header>
          <label class="outbound-allocation-quantity">
            <span>数量（{{ allocationLine.item.unit }}） *</span>
            <input
              v-model="allocationDraft.quantity"
              data-outbound-allocation-quantity
              inputmode="decimal"
              type="number"
              min="0.01"
              step="0.01"
              :class="{ error: validation && !validQuantity(allocationDraft.quantity) }"
              :aria-label="`${allocationLine.item.name} 出库数量`"
            />
          </label>
        </section>
        <section class="outbound-allocation-section">
          <header><strong>扣减方式</strong><span>选择实际出库时扣减库存的规则。</span></header>
          <fieldset class="outbound-allocation-editor">
            <label>
              <input v-model="allocationDraft.mode" type="radio" value="fifo" />
              <span class="outbound-radio-indicator" aria-hidden="true"></span>
              <span><strong>按先进先出分配</strong><small>从指定库位或全部库存按 FIFO 扣减。</small></span>
            </label>
            <label>
              <input v-model="allocationDraft.mode" type="radio" value="specific_batch" />
              <span class="outbound-radio-indicator" aria-hidden="true"></span>
              <span><strong>指定批次</strong><small>从选定批次扣减，库位随批次确定。</small></span>
            </label>
          </fieldset>
        </section>
        <section v-if="allocationDraft.mode === 'fifo'" class="outbound-allocation-section">
          <header><strong>扣减范围</strong><span>不限制时，审批可从全部库位按 FIFO 分配。</span></header>
          <label class="outbound-location">
            <span>限制库位（可选）</span>
            <SelectControl v-model="allocationDraft.locationId" aria-label="限制库位" compact>
              <option :value="null">全部库位</option>
              <option v-for="location in locations" :key="location.id" :value="location.id">
                {{ location.name }}
              </option>
            </SelectControl>
            <small v-if="locationError">{{ locationError }}，仍可按全部库位 FIFO 分配。</small>
          </label>
        </section>
        <section v-else class="outbound-allocation-section">
          <header><strong>选择批次</strong><span>批次可用数量仅为当前快照，实际出库时仍会校验库存。</span></header>
          <div class="outbound-batches" @scroll.passive="handleBatchScroll">
            <div v-for="batch in batches" :key="batch.id" class="outbound-batch">
              <label>
                <input v-model="allocationDraft.batchId" type="radio" :value="batch.id" />
                <span class="outbound-radio-indicator" aria-hidden="true"></span>
                <span>
                  <strong>{{ batch.batch_no }}</strong>
                  <small>
                    {{ batch.location_name }} · 剩余 {{ batch.remaining_quantity }} {{ allocationLine.item.unit }}
                    {{ batch.expires_at ? ` · 有效期 ${batch.expires_at}` : "" }} · 成本 ¥{{ formatMoney(batch.unit_cost) }} / {{ allocationLine.item.unit }}
                  </small>
                </span>
              </label>
            </div>
            <p v-if="batchError">
              {{ batchError }} <button class="text-button" type="button" @click="resetBatches">重试</button>
            </p>
            <p v-else-if="batchPending">正在加载批次…</p>
            <p v-else-if="batchMore">继续向下滚动加载</p>
            <p v-else>已加载全部批次</p>
          </div>
        </section>
        <p class="outbound-cost-hint">{{ allocationCostHint }}</p>
      </template>
      <template #actions>
        <button class="secondary-button" type="button" @click="closeAllocation">暂存并关闭</button>
        <button class="primary-button" type="button" @click="completeAllocationAndContinue">
          完成并继续添加
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="confirmMode !== null"
      :title="
        confirmMode === 'clear'
          ? '清空出库草稿？'
          : confirmMode === 'leave'
            ? '离开当前页面？'
            : canDirectOutbound
              ? '确认直接出库？'
              : '确认提交审核？'
      "
      :description="
        confirmMode === 'submit'
          ? canDirectOutbound
            ? '确认后会立即扣减库存并写入出库流水。'
            : '提交后进入待审批，库存不会立即扣减。'
          : confirmMode === 'leave'
            ? '当前草稿已自动保存在本机，离开后仍可恢复。'
            : '所有未提交内容将从本机删除。'
      "
      :busy="submitting"
      @close="cancelConfirmation"
    >
      <template v-if="confirmMode === 'submit'">
        <dl class="outbound-confirm">
          <div><dt>出库去向</dt><dd>{{ destination }}</dd></div>
          <div><dt>明细</dt><dd>{{ lines.length }} 条 · {{ quantitySummary }}</dd></div>
          <div><dt>预计出库成本</dt><dd>{{ confirmCostLabel }}</dd></div>
        </dl>
      </template>
      <p v-else>{{ confirmMode === "leave" ? "确认离开当前出库流程吗？" : "此操作无法撤销。" }}</p>
      <template #actions>
        <button class="secondary-button" type="button" :disabled="submitting" @click="cancelConfirmation">
          {{ confirmMode === "submit" ? "返回检查" : "取消" }}
        </button>
        <button
          :class="confirmMode === 'clear' ? 'danger-button' : 'primary-button'"
          type="button"
          :disabled="submitting"
          @click="confirmAction"
        >
          {{
            submitting
              ? "正在提交…"
              : confirmMode === "clear"
                ? "确认清空"
                : confirmMode === "leave"
                  ? "确认离开"
                  : canDirectOutbound
                    ? "确认直接出库"
                    : "确认提交审核"
          }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRouter } from "vue-router";
import { listItemBatches, type ItemBatchStockResponse, type ItemOptionResponse } from "../api/items";
import { listLocations, type LocationResponse } from "../api/locations";
import { createOutbound } from "../api/outbound";
import { approveOutboundOrder } from "../api/stockApprovals";
import { ApiError } from "../api/errors";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { authSession } from "../auth/session";
import AuthenticatedImage from "../components/attributes/AuthenticatedImage.vue";
import SelectControl from "../components/forms/SelectControl.vue";
import ItemSelectionDialog from "../components/items/ItemSelectionDialog.vue";
import ModalDialog from "../components/ModalDialog.vue";
import { useInboundItemCatalog } from "../composables/useInboundItemCatalog";
import { useOutboundDraftPersistence } from "../composables/useOutboundDraftPersistence";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";
import {
  buildOutboundRequest,
  createOutboundDraftLine,
  emptyCostEstimate,
  estimateFifoCost,
  estimateSpecificBatchCost,
  lineError,
  type OutboundAllocationMode,
  type OutboundCostEstimate,
  type OutboundDraftLine,
} from "./outbound-draft/model";
import { formatMoney } from "./inbound-draft/presentation";
import "./OutboundDraftPage.scss";

interface CostBatchSnapshot {
  state: "loading" | "complete" | "failed";
  batches: ItemBatchStockResponse[];
}

const router = useRouter();
const destination = ref("");
const notes = ref("");
const notesOpen = ref(false);
const lines = ref<OutboundDraftLine[]>([]);
const validation = ref(false);
const submitting = ref(false);
const confirmMode = ref<"clear" | "submit" | "leave" | null>(null);
const destinationInput = ref<HTMLInputElement | null>(null);
const locations = ref<LocationResponse[]>([]);
const locationError = ref("");
const itemPickerOpen = ref(false);
const allocationLine = ref<OutboundDraftLine | null>(null);
const allocationDraft = ref({
  quantity: "",
  mode: "fifo" as OutboundAllocationMode,
  batchId: null as number | null,
  locationId: null as number | null,
});
const batches = ref<ItemBatchStockResponse[]>([]);
const batchLoading = ref(false);
const batchError = ref("");
const batchPage = ref(0);
const batchPages = ref(0);
const costBatchSnapshots = ref<Record<number, CostBatchSnapshot>>({});
let batchController: AbortController | null = null;
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;
let pendingPickerItem: ItemOptionResponse | null = null;
let openPickerAfterAllocation = false;
const costBatchControllers = new Map<number, AbortController>();

const canReadItems = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.outboundCreate),
);
const hasDraft = computed(
  () => lines.value.length > 0 || !!destination.value.trim() || !!notes.value.trim(),
);
const selectedItemIds = computed<ReadonlySet<number>>(
  () => new Set(lines.value.map((line) => line.item.id)),
);
const incompleteLine = computed(() => lines.value.find((line) => lineError(line) !== null) ?? null);
const canAddItem = computed(() => allocationLine.value === null && incompleteLine.value === null);
const addItemDisabledReason = computed(() => {
  if (allocationLine.value !== null) return "请先完成或暂存当前打开的出库明细。";
  if (incompleteLine.value) return `请先完成“${incompleteLine.value.item.name}”的出库明细。`;
  return "";
});
const quantitySummary = computed(
  () =>
    Array.from(
      lines.value.reduce((map, line) => {
        const quantity = Number(line.quantity);
        if (quantity > 0) map.set(line.item.unit, (map.get(line.item.unit) || 0) + quantity);
        return map;
      }, new Map<string, number>()),
    )
      .map(([unit, quantity]) => `${quantity} ${unit}`)
      .join("、") || "未填写数量",
);
const batchMore = computed(() => batchPage.value < batchPages.value);
const batchPending = useStablePendingIndicator(batchLoading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const costSummary = computed(() => {
  const estimates = lines.value
    .filter((line) => Number(line.quantity) > 0)
    .map((line) => lineCostEstimate(line));
  if (!estimates.length) return emptyCostEstimate("idle");
  if (estimates.every((estimate) => estimate.state === "complete"))
    return {
      state: "complete" as const,
      amount: estimates.reduce((total, estimate) => total + (estimate.amount ?? 0), 0),
    };
  if (estimates.some((estimate) => estimate.state === "loading" || estimate.state === "idle"))
    return { state: "loading" as const, amount: null };
  if (estimates.some((estimate) => estimate.state === "insufficient"))
    return { state: "insufficient" as const, amount: null };
  return { state: "failed" as const, amount: null };
});
const allocationCostEstimate = computed(() => {
  const line = allocationLine.value;
  if (!line) return emptyCostEstimate("idle");
  return lineCostEstimate({
    ...line,
    quantity: allocationDraft.value.quantity,
    allocationMode: allocationDraft.value.mode,
    batchId: allocationDraft.value.batchId,
    locationId: allocationDraft.value.locationId,
  });
});
const allocationCostHint = computed(() => costEstimateDetail(allocationCostEstimate.value));
const confirmCostLabel = computed(() =>
  costSummary.value.state === "complete"
    ? `¥${formatMoney(costSummary.value.amount ?? 0)}`
    : "实际出库时按扣减批次确认",
);
const canDirectOutbound = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.outboundApprove),
);

const {
  items,
  searchInput,
  loadingItems,
  itemError,
  itemList,
  itemsExhausted,
  resetItems,
  loadNextItems,
  applySearch,
  handleItemScroll,
} = useInboundItemCatalog((error) =>
  error instanceof ApiError && error.status === 403 ? "当前账号没有读取物品的权限" : "加载物品失败，请重试",
);
const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useOutboundDraftPersistence(
  destination,
  notes,
  notesOpen,
  lines,
  hasDraft,
);

onMounted(async () => {
  if (restoreDraft()) notice.info("已恢复上次未提交的出库草稿");
  resumeDraftSaving();
  requestCostEstimates();
  void resetItems();
  if (hasPermission(authSession.value?.user.permissions, stockPermissions.locationRead)) {
    try {
      locations.value = await listLocations({});
    } catch {
      locationError.value = "无法加载库位";
    }
  }
});

onBeforeUnmount(() => {
  batchController?.abort();
  costBatchControllers.forEach((controller) => controller.abort());
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
});

onBeforeRouteLeave(() => {
  if (!hasDraft.value) return true;
  confirmMode.value = "leave";
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

watch(
  () => allocationDraft.value.mode,
  (mode) => {
    if (mode === "specific_batch" && allocationLine.value && !batches.value.length) void resetBatches();
  },
);
watch(lines, requestCostEstimates, { deep: true });

function openItemPicker(): void {
  if (!canAddItem.value) {
    if (incompleteLine.value)
      notice.warning(`请先完成“${incompleteLine.value.item.name}”的出库明细。`);
    return;
  }
  itemPickerOpen.value = true;
  void resetItems();
}

function closeItemPicker(): void {
  pendingPickerItem = null;
  itemPickerOpen.value = false;
}

function handleItemSelected(item: ItemOptionResponse): void {
  pendingPickerItem = item;
  itemPickerOpen.value = false;
}

function handleItemPickerAfterClose(): void {
  const item = pendingPickerItem;
  pendingPickerItem = null;
  if (!item) return;
  const existing = lines.value.find((line) => line.item.id === item.id);
  const line = existing ?? createOutboundDraftLine(item);
  if (!existing) lines.value.push(line);
  openAllocation(line);
}

function remove(id: string): void {
  if (allocationLine.value?.lineId === id) closeAllocation();
  const line = lines.value.find((candidate) => candidate.lineId === id);
  lines.value = lines.value.filter((candidate) => candidate.lineId !== id);
  if (line && !lines.value.some((candidate) => candidate.item.id === line.item.id)) {
    costBatchControllers.get(line.item.id)?.abort();
    costBatchControllers.delete(line.item.id);
    const remaining = { ...costBatchSnapshots.value };
    delete remaining[line.item.id];
    costBatchSnapshots.value = remaining;
  }
}

function requestCostEstimates(): void {
  for (const line of lines.value) if (Number(line.quantity) > 0) void loadCostBatches(line.item.id);
}

function costSnapshot(itemId: number): CostBatchSnapshot | undefined {
  return costBatchSnapshots.value[itemId];
}

function lineCostEstimate(line: OutboundDraftLine): OutboundCostEstimate {
  if (!validQuantity(line.quantity)) return emptyCostEstimate("idle");
  const snapshot = costSnapshot(line.item.id);
  if (!snapshot || snapshot.state === "loading") return emptyCostEstimate("loading");
  if (snapshot.state === "failed") return emptyCostEstimate("failed");
  return line.allocationMode === "specific_batch"
    ? estimateSpecificBatchCost(line, findBatch(line))
    : estimateFifoCost(line, snapshot.batches);
}

function costEstimateLabel(line: OutboundDraftLine): string {
  const estimate = lineCostEstimate(line);
  if (estimate.state === "complete")
    return `预计 ¥${formatMoney(estimate.amount ?? 0)}${estimate.allocationCount > 1 ? ` · ${estimate.allocationCount} 个批次` : ""}`;
  if (estimate.state === "insufficient") return "成本以实际出库为准";
  if (estimate.state === "failed") return "暂无法估算成本";
  return estimate.state === "loading" ? "正在估算成本…" : "填写数量后估算成本";
}

function costEstimateDetail(estimate: OutboundCostEstimate): string {
  if (estimate.state === "complete")
    return `预计出库成本 ¥${formatMoney(estimate.amount ?? 0)}${estimate.allocationCount > 1 ? `，按 ${estimate.allocationCount} 个批次分摊。` : "。"} 实际出库时会按实际库存重新校验。`;
  if (estimate.state === "insufficient") return "当前库存快照无法完整覆盖该数量，成本将在实际出库时按实际扣减批次确认。";
  if (estimate.state === "failed") return "暂无法读取批次成本；不影响提交，实际出库时会按实际库存处理。";
  if (estimate.state === "loading") return "正在按当前批次余额估算成本…";
  return "填写数量后，将按当前批次余额估算成本。";
}

async function loadCostBatches(itemId: number): Promise<void> {
  const existing = costSnapshot(itemId);
  if (existing?.state === "loading" || existing?.state === "complete") return;
  const controller = new AbortController();
  costBatchControllers.get(itemId)?.abort();
  costBatchControllers.set(itemId, controller);
  let page = 1;
  let totalPages = 1;
  let snapshotBatches: ItemBatchStockResponse[] = [];
  costBatchSnapshots.value = {
    ...costBatchSnapshots.value,
    [itemId]: { state: "loading", batches: snapshotBatches },
  };
  try {
    while (page <= totalPages) {
      const response = await listItemBatches(itemId, page, 50, controller.signal);
      if (costBatchControllers.get(itemId) !== controller) return;
      snapshotBatches = [
        ...snapshotBatches,
        ...response.items.filter((batch) => !snapshotBatches.some((candidate) => candidate.id === batch.id)),
      ];
      page = response.page + 1;
      totalPages = response.total_pages;
      costBatchSnapshots.value = {
        ...costBatchSnapshots.value,
        [itemId]: { state: "loading", batches: snapshotBatches },
      };
    }
    costBatchSnapshots.value = {
      ...costBatchSnapshots.value,
      [itemId]: { state: "complete", batches: snapshotBatches },
    };
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    if (costBatchControllers.get(itemId) === controller)
      costBatchSnapshots.value = {
        ...costBatchSnapshots.value,
        [itemId]: { state: "failed", batches: snapshotBatches },
      };
  } finally {
    if (costBatchControllers.get(itemId) === controller) costBatchControllers.delete(itemId);
  }
}

function setItemList(element: unknown): void {
  itemList.value = element instanceof HTMLElement ? element : null;
}

function findBatch(line: OutboundDraftLine): ItemBatchStockResponse | undefined {
  if (line.batchId === null) return undefined;
  return (
    batches.value.find((batch) => batch.id === line.batchId) ??
    costSnapshot(line.item.id)?.batches.find((batch) => batch.id === line.batchId)
  );
}

function allocationSummary(line: OutboundDraftLine): string {
  if (line.allocationMode === "fifo")
    return line.locationId ? `按 FIFO 分配 · 库位 #${line.locationId}` : "按 FIFO 分配 · 全部库位";
  const batch = findBatch(line);
  return batch ? `批次 ${batch.batch_no} · ${batch.location_name}` : line.batchId ? `指定批次 #${line.batchId}` : "尚未选择批次";
}

function allocationPrimary(line: OutboundDraftLine): string {
  if (lineError(line)) return "待配置";
  return line.allocationMode === "fifo" ? "FIFO 分配" : "指定批次";
}

function allocationSecondary(line: OutboundDraftLine): string {
  if (line.allocationMode === "fifo") {
    const location = locations.value.find((candidate) => candidate.id === line.locationId);
    return location ? `库位：${location.name}` : "全部库位";
  }
  const batch = findBatch(line);
  return batch ? `${batch.batch_no} · ${batch.location_name}` : line.batchId ? `批次 #${line.batchId}` : "尚未选择批次";
}

function quantityLabel(line: OutboundDraftLine): string {
  return validQuantity(line.quantity) ? `${line.quantity} ${line.item.unit}` : "待填写";
}

function openAllocation(line: OutboundDraftLine): void {
  void loadCostBatches(line.item.id);
  allocationLine.value = line;
  allocationDraft.value = {
    quantity: line.quantity,
    mode: line.allocationMode,
    batchId: line.batchId,
    locationId: line.locationId,
  };
  batches.value = [];
  batchPage.value = 0;
  batchPages.value = 0;
  batchError.value = "";
  if (line.allocationMode === "specific_batch") void resetBatches();
}

function writeAllocationDraft(line: OutboundDraftLine): void {
  line.quantity = allocationDraft.value.quantity;
  line.allocationMode = allocationDraft.value.mode;
  if (allocationDraft.value.mode === "specific_batch") {
    line.batchId = allocationDraft.value.batchId;
    line.locationId = findBatch({ ...line, batchId: allocationDraft.value.batchId })?.location_id ?? allocationDraft.value.locationId;
  } else {
    line.batchId = null;
    line.locationId = allocationDraft.value.locationId;
  }
}

function dismissAllocation(): void {
  batchController?.abort();
  allocationLine.value = null;
}

function closeAllocation(): void {
  openPickerAfterAllocation = false;
  if (allocationLine.value) writeAllocationDraft(allocationLine.value);
  dismissAllocation();
}

function completeAllocationAndContinue(): void {
  const line = allocationLine.value;
  if (!line) return;
  const candidate = {
    ...line,
    quantity: allocationDraft.value.quantity,
    allocationMode: allocationDraft.value.mode,
    batchId: allocationDraft.value.batchId,
    locationId: allocationDraft.value.locationId,
  };
  validation.value = true;
  if (lineError(candidate)) {
    notice.warning("当前出库明细尚未完成", { detail: lineError(candidate) ?? undefined });
    void nextTick(() => document.querySelector<HTMLElement>("[data-outbound-allocation-quantity]")?.focus());
    return;
  }
  writeAllocationDraft(line);
  openPickerAfterAllocation = true;
  dismissAllocation();
}

function handleAllocationAfterClose(): void {
  if (!openPickerAfterAllocation) return;
  openPickerAfterAllocation = false;
  itemPickerOpen.value = true;
  void resetItems();
}

function handleBatchScroll(event: Event): void {
  const element = event.currentTarget as HTMLElement;
  if (element.scrollHeight - element.scrollTop - element.clientHeight < 100) void loadBatches();
}

async function resetBatches(): Promise<void> {
  if (allocationLine.value && costSnapshot(allocationLine.value.item.id)?.state === "failed")
    void retryCostBatches(allocationLine.value.item.id);
  batches.value = [];
  batchPage.value = 0;
  batchPages.value = 0;
  await loadBatches();
}

async function retryCostBatches(itemId: number): Promise<void> {
  const remaining = { ...costBatchSnapshots.value };
  delete remaining[itemId];
  costBatchSnapshots.value = remaining;
  await loadCostBatches(itemId);
}

async function loadBatches(): Promise<void> {
  if (!allocationLine.value || batchLoading.value || (!batchMore.value && batchPage.value > 0)) return;
  batchController?.abort();
  const controller = new AbortController();
  batchController = controller;
  batchLoading.value = true;
  batchError.value = "";
  try {
    const response = await listItemBatches(
      allocationLine.value.item.id,
      batchPage.value + 1,
      20,
      controller.signal,
    );
    if (batchController !== controller) return;
    batches.value = [
      ...batches.value,
      ...response.items.filter((batch) => !batches.value.some((candidate) => candidate.id === batch.id)),
    ];
    batchPage.value = response.page;
    batchPages.value = response.total_pages;
  } catch (error) {
    if (!(error instanceof DOMException && error.name === "AbortError"))
      batchError.value = error instanceof ApiError && error.status === 403 ? "无权读取库存批次" : "加载批次失败";
  } finally {
    if (batchController === controller) {
      batchController = null;
      batchLoading.value = false;
    }
  }
}

function validQuantity(value: string): boolean {
  const number = Number(value);
  return Number.isFinite(number) && number > 0;
}

async function review(): Promise<void> {
  validation.value = true;
  if (!destination.value.trim()) {
    notice.warning("请填写出库去向");
    destinationInput.value?.focus();
    return;
  }
  if (!lines.value.length) {
    notice.warning("请至少添加一条出库明细");
    openItemPicker();
    return;
  }
  const bad = lines.value.find((line) => lineError(line));
  if (bad) {
    notice.warning("出库明细尚未填写完整", { detail: `请检查“${bad.item.name}”。` });
    openAllocation(bad);
    return;
  }
  confirmMode.value = "submit";
}

function cancelConfirmation(): void {
  if (submitting.value) return;
  if (confirmMode.value === "leave") pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
  confirmMode.value = null;
}

async function confirmAction(): Promise<void> {
  if (confirmMode.value === "leave") {
    const resolve = pendingLeaveResolution;
    pendingLeaveResolution = null;
    confirmMode.value = null;
    resolve?.(true);
    return;
  }
  if (confirmMode.value === "clear") {
    destination.value = "";
    notes.value = "";
    notesOpen.value = false;
    lines.value = [];
    validation.value = false;
    removePersistedDraft();
    confirmMode.value = null;
    return;
  }
  submitting.value = true;
  try {
    const order = await createOutbound(buildOutboundRequest(destination.value, notes.value, lines.value));
    if (canDirectOutbound.value) await approveOutboundOrder(order.id);
    notice.success(canDirectOutbound.value ? "出库成功" : "出库单已提交", {
      detail: canDirectOutbound.value
        ? `单号 #${order.id} 已完成出库，库存已扣减。`
        : `单号 #${order.id} 已进入待审批状态，库存未扣减。`,
      onClick: hasPermission(authSession.value?.user.permissions, stockPermissions.outboundRead)
        ? () => router.push({ name: "outbound-orders" })
        : undefined,
    });
    destination.value = "";
    notes.value = "";
    notesOpen.value = false;
    lines.value = [];
    validation.value = false;
    removePersistedDraft();
    confirmMode.value = null;
  } catch (error) {
    notice.error(canDirectOutbound.value ? "直接出库失败" : "提交出库单失败", {
      detail: error instanceof ApiError ? error.message : "请检查网络后重试",
    });
  } finally {
    submitting.value = false;
  }
}
</script>
