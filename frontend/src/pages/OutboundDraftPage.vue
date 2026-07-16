<!-- 本页面拥有待审批出库单的选品、草稿、批次分配和提交会话；不执行审批或库存扣减。 -->
<template>
  <section class="route-page outbound-draft-page">
    <header class="content-header outbound-draft-page__header">
      <div class="outbound-title">
        <div class="outbound-progress">
          <span :class="{ active: step === 'catalog' }">1</span><i /><span
            :class="{ active: step === 'draft' }"
            >2</span
          >
        </div>
        <h1>{{ $route.meta.title }}</h1>
      </div>
      <div v-if="step === 'draft'" class="outbound-summary">
        <span
          ><strong>{{ lines.length }}</strong> 条明细</span
        ><span>·</span>
        <span>{{ itemCount }} 个物品</span>
        <template v-if="costSummary.state === 'complete'">
          <span>·</span
          ><span
            >预计成本 <strong>¥{{ formatMoney(costSummary.amount ?? 0) }}</strong></span
          >
        </template>
        <template v-else-if="costSummary.state === 'loading'">
          <span>·</span><span>正在估算成本…</span>
        </template>
        <template
          v-else-if="costSummary.state === 'insufficient' || costSummary.state === 'failed'"
        >
          <span>·</span><span>成本以实际出库为准</span>
        </template>
      </div>
      <div class="outbound-actions">
        <button
          class="text-button outbound-clear"
          :disabled="!hasDraft || submitting"
          @click="confirmMode = 'clear'"
        >
          清空草稿</button
        ><button
          v-if="step === 'draft'"
          class="primary-button"
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
      <button class="secondary-button" @click="router.back()">返回</button>
    </section>
    <section v-else-if="step === 'catalog'" class="outbound-workspace">
      <header>
        <div>
          <h2>选择出库物品</h2>
          <p>加入后再填写数量及扣减方式。</p>
        </div>
        <button class="secondary-button" :disabled="!lines.length" @click="step = 'draft'">
          下一步：填写单据
        </button>
      </header>
      <SearchField
        v-model="searchInput"
        label="搜索物品"
        name="outbound_item_search"
        placeholder="名称或编号"
        hide-label
        @search="applySearch"
      />
      <div v-if="itemError && !items.length" class="outbound-state">
        <strong>{{ itemError }}</strong
        ><button class="text-button" @click="resetItems">重试</button>
      </div>
      <div v-else :ref="setItemList" class="outbound-catalog" @scroll.passive="handleItemScroll">
        <article
          v-for="item in items"
          :key="item.id"
          :class="{ 'outbound-catalog__item--selected': selected(item.id) }"
        >
          <AuthenticatedImage
            :file-id="item.image_file_id"
            :alt="`${item.name} 主图`"
            :size="38"
            previewable
          />
          <div>
            <strong>{{ item.name }}</strong
            ><small>{{ item.sku }} · {{ item.unit }}</small>
          </div>
          <button
            class="outbound-catalog__toggle"
            :class="{ 'outbound-catalog__toggle--selected': selected(item.id) }"
            type="button"
            :aria-label="
              selected(item.id) ? `将 ${item.name} 移出出库申请` : `将 ${item.name} 加入出库申请`
            "
            :aria-pressed="selected(item.id)"
            :title="selected(item.id) ? '移出申请' : '加入申请'"
            @click="toggle(item)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path v-if="selected(item.id)" d="M5 12h14" />
              <path v-else d="M12 5v14M5 12h14" />
            </svg>
          </button>
        </article>
        <div class="outbound-tail">{{ catalogState }}</div>
      </div>
    </section>
    <section v-else class="outbound-workspace outbound-workspace--draft">
      <header>
        <div>
          <h2>填写出库单</h2>
          <p>
            {{
              canDirectOutbound
                ? "确认后立即扣减库存并写入出库流水。"
                : "提交后进入待审批，库存不会立即扣减。"
            }}
          </p>
        </div>
        <button class="secondary-button" @click="step = 'catalog'">上一步：选择物品</button>
      </header>
      <div class="outbound-meta">
        <label
          ><span>出库去向 *</span
          ><input
            ref="destinationInput"
            v-model="destination"
            name="outbound_destination"
            :class="{ error: validation && !destination.trim() }"
            maxlength="128"
            placeholder="客户 / 部门 / 项目" /></label
        ><button
          class="icon-button outbound-notes-toggle"
          :class="{ 'outbound-notes-toggle--filled': notes.trim() }"
          type="button"
          :title="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          :aria-label="notesOpen ? '收起备注' : notes.trim() ? '备注已填写' : '添加备注'"
          @click="notesOpen = !notesOpen"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 4h14v12H9l-4 4V4Z" />
            <path d="M8 8h8M8 12h5" />
          </svg></button
        ><label v-if="notesOpen" class="outbound-notes"
          ><span>备注</span
          ><input
            v-model="notes"
            name="outbound_notes"
            maxlength="1024"
            placeholder="可选，记录出库说明"
        /></label>
      </div>
      <div class="outbound-lines">
        <article
          v-for="(line, index) in lines"
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
              <strong>{{ line.item.name }}</strong
              ><small>{{ line.item.sku }} · {{ line.item.unit }} · 明细 {{ index + 1 }}</small
              ><small>分类：{{ line.item.category_name || "未分类" }}</small>
            </div>
          </div>
          <label
            ><span>数量（{{ line.item.unit }}）</span
            ><input
              v-model="line.quantity"
              :name="`outbound_quantity_${line.lineId}`"
              inputmode="decimal"
              type="number"
              min="0.01"
              step="0.01"
              :aria-label="`${line.item.name} 出库数量`"
          /></label>
          <button
            class="outbound-allocation"
            type="button"
            :data-line-action="line.lineId"
            :aria-label="`${line.item.name}：${allocationSummary(line)}，设置扣减批次与库位`"
            @click="openAllocation(line)"
          >
            <span class="outbound-allocation__status">
              <small>扣减方式</small>
              <strong>{{ allocationPrimary(line) }}</strong>
              <span>{{ allocationSecondary(line) }}</span>
              <span class="outbound-allocation__cost">{{ costEstimateLabel(line) }}</span>
            </span>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 20h4l11-11-4-4L4 16v4Z" />
            </svg>
          </button>
          <button
            class="icon-button"
            :aria-label="`移除 ${line.item.name}`"
            title="移除明细"
            @click="remove(line.lineId)"
          >
            ×
          </button>
        </article>
      </div>
    </section>
    <ModalDialog
      :open="allocationLine !== null"
      title="设置扣减批次与库位"
      wide
      @close="closeAllocation"
    >
      <template #context
        ><div v-if="allocationLine" class="outbound-allocation-context">
          <strong>{{ allocationLine.item.name }}</strong
          ><span>{{ allocationLine.item.sku }} · {{ allocationLine.item.unit }}</span>
        </div></template
      >
      <template v-if="allocationLine"
        ><section class="outbound-allocation-section">
          <header><strong>扣减方式</strong><span>选择实际出库时扣减库存的规则。</span></header>
          <fieldset class="outbound-allocation-editor">
            <label
              ><input v-model="allocationDraft.mode" type="radio" value="fifo" /><span
                ><strong>按先进先出分配</strong
                ><small>实际出库时从指定库位或全部库存按 FIFO 扣减。</small></span
              ></label
            ><label
              ><input v-model="allocationDraft.mode" type="radio" value="specific_batch" /><span
                ><strong>指定批次</strong
                ><small>实际出库时从选定批次扣减，库位随批次确定。</small></span
              ></label
            >
          </fieldset>
        </section>
        <section v-if="allocationDraft.mode === 'fifo'" class="outbound-allocation-section">
          <header>
            <strong>扣减范围</strong><span>不限制时，审批可从全部库位按 FIFO 分配。</span>
          </header>
          <label class="outbound-location"
            ><span>限制库位（可选）</span
            ><select v-model="allocationDraft.locationId">
              <option :value="null">全部库位</option>
              <option v-for="location in locations" :key="location.id" :value="location.id">
                {{ location.name }}
              </option></select
            ><small v-if="locationError"
              >{{ locationError }}，仍可按全部库位 FIFO 分配。</small
            ></label
          >
        </section>
        <section v-else class="outbound-allocation-section">
          <header>
            <strong>选择批次</strong><span>批次可用数量仅为当前快照，实际出库时仍会校验库存。</span>
          </header>
          <div class="outbound-batches" @scroll.passive="handleBatchScroll">
            <div v-for="batch in batches" :key="batch.id" class="outbound-batch">
              <label
                ><input v-model="allocationDraft.batchId" type="radio" :value="batch.id" /><span
                  ><strong>{{ batch.batch_no }}</strong
                  ><small
                    >{{ batch.location_name }} · 剩余 {{ batch.remaining_quantity }}
                    {{ allocationLine.item.unit
                    }}{{ batch.expires_at ? ` · 有效期 ${batch.expires_at}` : "" }} · 成本 ¥{{
                      formatMoney(batch.unit_cost)
                    }}
                    / {{ allocationLine.item.unit }}</small
                  ></span
                ></label
              >
            </div>
            <p v-if="batchError">
              {{ batchError }}
              <button class="text-button" @click="resetBatches">重试</button>
            </p>
            <p v-else-if="batchPending">正在加载批次…</p>
            <p v-else-if="batchMore">继续向下滚动加载</p>
            <p v-else>已加载全部批次</p>
          </div>
        </section>
        <p class="outbound-cost-hint">{{ allocationCostHint }}</p></template
      >
      <template #actions
        ><button class="secondary-button" @click="closeAllocation">取消</button
        ><button class="primary-button" @click="applyAllocation">应用</button></template
      >
    </ModalDialog>
    <ModalDialog
      :open="confirmMode !== null"
      :title="
        confirmMode === 'clear'
          ? '清空出库草稿？'
          : canDirectOutbound
            ? '确认直接出库？'
            : '确认提交审核？'
      "
      :description="
        confirmMode === 'submit'
          ? canDirectOutbound
            ? '确认后会立即扣减库存并写入出库流水。'
            : '提交后进入待审批，库存不会立即扣减。'
          : '所有未提交内容将从本机删除。'
      "
      :busy="submitting"
      @close="confirmMode = null"
      ><template v-if="confirmMode === 'submit'"
        ><dl class="outbound-confirm">
          <div>
            <dt>出库去向</dt>
            <dd>{{ destination }}</dd>
          </div>
          <div>
            <dt>明细</dt>
            <dd>{{ lines.length }} 条 · {{ quantitySummary }}</dd>
          </div>
          <div>
            <dt>预计出库成本</dt>
            <dd>{{ confirmCostLabel }}</dd>
          </div>
        </dl></template
      ><template #actions
        ><button class="secondary-button" :disabled="submitting" @click="confirmMode = null">
          {{ confirmMode === "submit" ? "返回检查" : "取消" }}</button
        ><button
          :class="confirmMode === 'clear' ? 'danger-button' : 'primary-button'"
          :disabled="submitting"
          @click="confirmAction"
        >
          {{
            submitting
              ? "正在提交…"
              : confirmMode === "clear"
                ? "确认清空"
                : canDirectOutbound
                  ? "确认直接出库"
                  : "确认提交审核"
          }}
        </button></template
      ></ModalDialog
    >
  </section>
</template>
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRouter } from "vue-router";
import {
  listItemBatches,
  type ItemBatchStockResponse,
  type ItemOptionResponse,
} from "../api/items";
import { listLocations, type LocationResponse } from "../api/locations";
import { createOutbound } from "../api/outbound";
import { approveOutboundOrder } from "../api/stockApprovals";
import { ApiError } from "../api/errors";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { authSession } from "../auth/session";
import AuthenticatedImage from "../components/attributes/AuthenticatedImage.vue";
import ModalDialog from "../components/ModalDialog.vue";
import SearchField from "../components/SearchField.vue";
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

/** 单物品的完整批次快照只用于本次草稿成本预估，离开页面后不持久化。 */
interface CostBatchSnapshot {
  state: "loading" | "complete" | "failed";
  batches: ItemBatchStockResponse[];
}

const router = useRouter(),
  step = ref<"catalog" | "draft">("catalog"),
  destination = ref(""),
  notes = ref(""),
  notesOpen = ref(false),
  lines = ref<OutboundDraftLine[]>([]),
  validation = ref(false),
  submitting = ref(false),
  confirmMode = ref<"clear" | "submit" | null>(null),
  destinationInput = ref<HTMLInputElement | null>(null),
  locations = ref<LocationResponse[]>([]),
  locationError = ref(""),
  allocationLine = ref<OutboundDraftLine | null>(null),
  allocationDraft = ref({
    mode: "fifo" as OutboundAllocationMode,
    batchId: null as number | null,
    locationId: null as number | null,
  }),
  batches = ref<ItemBatchStockResponse[]>([]),
  batchLoading = ref(false),
  batchError = ref(""),
  batchPage = ref(0),
  batchPages = ref(0),
  costBatchSnapshots = ref<Record<number, CostBatchSnapshot>>({});
let batchController: AbortController | null = null;
const costBatchControllers = new Map<number, AbortController>();
const canReadItems = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.outboundCreate),
  ),
  hasDraft = computed(
    () => lines.value.length > 0 || !!destination.value.trim() || !!notes.value.trim(),
  ),
  itemCount = computed(() => new Set(lines.value.map((l) => l.item.id)).size),
  quantitySummary = computed(
    () =>
      Array.from(
        lines.value.reduce((m, l) => {
          const n = Number(l.quantity);
          if (n > 0) m.set(l.item.unit, (m.get(l.item.unit) || 0) + n);
          return m;
        }, new Map<string, number>()),
      )
        .map(([u, n]) => `${n} ${u}`)
        .join("、") || "未填写数量",
  ),
  batchMore = computed(() => batchPage.value < batchPages.value),
  catalogState = computed(() =>
    loadingItems.value
      ? "正在加载物品…"
      : itemError.value
        ? "加载失败，请重试"
        : itemsExhausted.value
          ? "已加载全部物品"
          : "继续向下滚动加载",
  ),
  batchPending = useStablePendingIndicator(batchLoading, {
    showDelayMs: 200,
    minimumVisibleMs: 350,
  }),
  costSummary = computed(() => {
    const estimates = lines.value
      .filter((line) => Number(line.quantity) > 0)
      .map((line) => lineCostEstimate(line));
    if (!estimates.length) return emptyCostEstimate("idle");
    if (estimates.every((estimate) => estimate.state === "complete")) {
      return {
        state: "complete" as const,
        amount: estimates.reduce((total, estimate) => total + (estimate.amount ?? 0), 0),
      };
    }
    if (estimates.some((estimate) => estimate.state === "loading" || estimate.state === "idle"))
      return { state: "loading" as const, amount: null };
    if (estimates.some((estimate) => estimate.state === "insufficient"))
      return { state: "insufficient" as const, amount: null };
    return { state: "failed" as const, amount: null };
  }),
  allocationCostEstimate = computed(() => {
    const line = allocationLine.value;
    if (!line) return emptyCostEstimate("idle");
    return lineCostEstimate({
      ...line,
      allocationMode: allocationDraft.value.mode,
      batchId: allocationDraft.value.batchId,
      locationId: allocationDraft.value.locationId,
    });
  }),
  allocationCostHint = computed(() => costEstimateDetail(allocationCostEstimate.value)),
  confirmCostLabel = computed(() =>
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
  applySearch,
  handleItemScroll,
} = useInboundItemCatalog((e) =>
  e instanceof ApiError && e.status === 403 ? "当前账号没有读取物品的权限" : "加载物品失败，请重试",
);
const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useOutboundDraftPersistence(
  destination,
  notes,
  notesOpen,
  lines,
  hasDraft,
);
onMounted(async () => {
  if (restoreDraft()) {
    notice.info("已恢复上次未提交的出库草稿");
    step.value = lines.value.length ? "draft" : "catalog";
  }
  resumeDraftSaving();
  requestCostEstimates();
  void resetItems();
  if (hasPermission(authSession.value?.user.permissions, stockPermissions.locationRead))
    try {
      locations.value = await listLocations({});
    } catch {
      locationError.value = "无法加载库位";
    }
});
onBeforeUnmount(() => {
  batchController?.abort();
  costBatchControllers.forEach((controller) => controller.abort());
});
onBeforeRouteLeave(() =>
  hasDraft.value ? window.confirm("当前出库草稿已自动保存在本机，确认离开吗？") : true,
);
watch(
  () => allocationDraft.value.mode,
  (mode) => {
    if (mode === "specific_batch" && allocationLine.value && !batches.value.length)
      void resetBatches();
  },
);
watch([step, lines], requestCostEstimates, { deep: true });
function selected(id: number) {
  return lines.value.some((l) => l.item.id === id);
}
function toggle(item: ItemOptionResponse) {
  const line = lines.value.find((l) => l.item.id === item.id);
  if (line) remove(line.lineId);
  else lines.value.push(createOutboundDraftLine(item));
}
function remove(id: string) {
  lines.value = lines.value.filter((l) => l.lineId !== id);
}
function requestCostEstimates() {
  if (step.value !== "draft") return;
  for (const line of lines.value) {
    if (Number(line.quantity) > 0) void loadCostBatches(line.item.id);
  }
}
function costSnapshot(itemId: number): CostBatchSnapshot | undefined {
  return costBatchSnapshots.value[itemId];
}
function lineCostEstimate(line: OutboundDraftLine): OutboundCostEstimate {
  if (!Number.isFinite(Number(line.quantity)) || Number(line.quantity) <= 0)
    return emptyCostEstimate("idle");
  const snapshot = costSnapshot(line.item.id);
  if (!snapshot || snapshot.state === "loading") return emptyCostEstimate("loading");
  if (snapshot.state === "failed") return emptyCostEstimate("failed");
  return line.allocationMode === "specific_batch"
    ? estimateSpecificBatchCost(
        line,
        snapshot.batches.find((batch) => batch.id === line.batchId),
      )
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
  if (estimate.state === "insufficient")
    return "当前库存快照无法完整覆盖该数量，成本将在实际出库时按实际扣减批次确认。";
  if (estimate.state === "failed")
    return "暂无法读取批次成本；不影响提交，实际出库时会按实际库存处理。";
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
  let batches: ItemBatchStockResponse[] = [];
  costBatchSnapshots.value = {
    ...costBatchSnapshots.value,
    [itemId]: { state: "loading", batches },
  };
  try {
    while (page <= totalPages) {
      const response = await listItemBatches(itemId, page, 50, controller.signal);
      if (costBatchControllers.get(itemId) !== controller) return;
      batches = [
        ...batches,
        ...response.items.filter(
          (batch) => !batches.some((candidate) => candidate.id === batch.id),
        ),
      ];
      page = response.page + 1;
      totalPages = response.total_pages;
      costBatchSnapshots.value = {
        ...costBatchSnapshots.value,
        [itemId]: { state: "loading", batches },
      };
    }
    costBatchSnapshots.value = {
      ...costBatchSnapshots.value,
      [itemId]: { state: "complete", batches },
    };
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    if (costBatchControllers.get(itemId) === controller) {
      costBatchSnapshots.value = {
        ...costBatchSnapshots.value,
        [itemId]: { state: "failed", batches: [] },
      };
    }
  } finally {
    if (costBatchControllers.get(itemId) === controller) costBatchControllers.delete(itemId);
  }
}
function setItemList(element: unknown) {
  itemList.value = element instanceof HTMLElement ? element : null;
}
function allocationSummary(line: OutboundDraftLine) {
  if (line.allocationMode === "fifo")
    return line.locationId ? `按 FIFO 分配 · 库位 #${line.locationId}` : "按 FIFO 分配 · 全部库位";
  const batch = batches.value.find((b) => b.id === line.batchId);
  return batch
    ? `批次 ${batch.batch_no} · ${batch.location_name}`
    : line.batchId
      ? `指定批次 #${line.batchId}`
      : "尚未选择批次";
}
function allocationPrimary(line: OutboundDraftLine) {
  return line.allocationMode === "fifo" ? "FIFO 分配" : "指定批次";
}
function allocationSecondary(line: OutboundDraftLine) {
  if (line.allocationMode === "fifo") {
    const location = locations.value.find((candidate) => candidate.id === line.locationId);
    return location ? `库位：${location.name}` : "全部库位";
  }
  const batch = batches.value.find((candidate) => candidate.id === line.batchId);
  return batch
    ? `${batch.batch_no} · ${batch.location_name}`
    : line.batchId
      ? `批次 #${line.batchId}`
      : "尚未选择批次";
}
function openAllocation(line: OutboundDraftLine) {
  void loadCostBatches(line.item.id);
  allocationLine.value = line;
  allocationDraft.value = {
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
function closeAllocation() {
  batchController?.abort();
  allocationLine.value = null;
}
function handleBatchScroll(event: Event) {
  const el = event.currentTarget as HTMLElement;
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 100) void loadBatches();
}
async function resetBatches() {
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
async function loadBatches() {
  if (!allocationLine.value || batchLoading.value || (!batchMore.value && batchPage.value > 0))
    return;
  batchController?.abort();
  const controller = new AbortController();
  batchController = controller;
  batchLoading.value = true;
  batchError.value = "";
  try {
    const r = await listItemBatches(
      allocationLine.value.item.id,
      batchPage.value + 1,
      20,
      controller.signal,
    );
    if (batchController !== controller) return;
    batches.value = [
      ...batches.value,
      ...r.items.filter((b) => !batches.value.some((x) => x.id === b.id)),
    ];
    batchPage.value = r.page;
    batchPages.value = r.total_pages;
  } catch (e) {
    if (!(e instanceof DOMException && e.name === "AbortError"))
      batchError.value =
        e instanceof ApiError && e.status === 403 ? "无权读取库存批次" : "加载批次失败";
  } finally {
    if (batchController === controller) {
      batchController = null;
      batchLoading.value = false;
    }
  }
}
function applyAllocation() {
  const line = allocationLine.value;
  if (!line) return;
  line.allocationMode = allocationDraft.value.mode;
  line.batchId =
    allocationDraft.value.mode === "specific_batch" ? allocationDraft.value.batchId : null;
  line.locationId =
    allocationDraft.value.mode === "specific_batch"
      ? (batches.value.find((b) => b.id === allocationDraft.value.batchId)?.location_id ?? null)
      : allocationDraft.value.locationId;
  closeAllocation();
}
async function review() {
  validation.value = true;
  if (!destination.value.trim()) {
    notice.warning("请填写出库去向");
    destinationInput.value?.focus();
    return;
  }
  if (!lines.value.length) {
    step.value = "catalog";
    notice.warning("请至少添加一条出库明细");
    return;
  }
  const bad = lines.value.find(lineError);
  if (bad) {
    notice.warning("出库明细尚未填写完整", {
      detail: `请检查“${bad.item.name}”。`,
    });
    return;
  }
  confirmMode.value = "submit";
}
async function confirmAction() {
  if (confirmMode.value === "clear") {
    destination.value = "";
    notes.value = "";
    notesOpen.value = false;
    lines.value = [];
    validation.value = false;
    removePersistedDraft();
    confirmMode.value = null;
    step.value = "catalog";
    return;
  }
  submitting.value = true;
  try {
    const order = await createOutbound(
      buildOutboundRequest(destination.value, notes.value, lines.value),
    );
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
    step.value = "catalog";
    confirmMode.value = null;
  } catch (e) {
    notice.error(canDirectOutbound.value ? "直接出库失败" : "提交出库单失败", {
      detail: e instanceof ApiError ? e.message : "请检查网络后重试",
    });
  } finally {
    submitting.value = false;
  }
}
</script>
