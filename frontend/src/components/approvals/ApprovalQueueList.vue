<!-- 本组件拥有审批队列的桌面三段式和移动单列呈现；它不请求数据或执行审批。 -->
<template>
  <div class="approval-queue-table" role="table" :aria-label="$t('approvals.pendingOrders')">
    <div class="approval-queue-table__head" role="row">
      <span>{{ $t('approvals.orderAndContext', { context: $t(catalog.contextLabel) }) }}</span
      ><span>{{ $t('approvals.itemsAndStockRules') }}</span><span>{{ $t('approvals.waitingAndActions') }}</span>
    </div>
    <article
      v-for="record in records"
      :key="`${record.kind}-${record.order.id}`"
      class="approval-queue-table__row"
      role="row"
      tabindex="0"
      @click="emit('open', record)"
      @keydown.enter="emit('open', record)"
      @keydown.space.prevent="emit('open', record)"
    >
      <div role="cell">
        <strong>{{ orderLabel(record) }} #{{ record.order.id }}</strong
        ><span>{{ approvalContext(record) }}</span
        ><time :datetime="record.order.created_at">{{ formatDate(record.order.created_at) }}</time>
      </div>
      <div role="cell">
        <div class="approval-queue-item">
          <AuthenticatedImage
            :file-id="firstItem(record).item_image_file_id"
            :alt="$t('approvals.itemMainImage', { name: firstItem(record).item_name })"
            :size="38"
            previewable
            @click.stop
            @keydown.stop
          />
          <div>
            <strong>{{ firstItem(record).item_name }}</strong
            ><small>{{ firstItem(record).item_sku }} · {{ itemSummary(record) }}</small>
          </div>
        </div>
        <span>{{ $t('approvals.itemLines', { n: record.order.items.length }) }} · {{ ruleSummary(record) }}</span>
      </div>
      <div class="approval-queue-table__decision" role="cell">
        <span class="approval-status">{{ $t('approvals.pending') }}</span
        ><span>{{ waitingLabel(record.order.created_at) }}</span
        ><button
          class="secondary-button approval-detail-button"
          type="button"
          :title="detailButtonTitle(record)"
          :aria-label="detailButtonAriaLabel(record)"
          @click.stop="emit('open', record)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="8" />
            <path d="M12 11v5M12 8h.01" />
          </svg>
          <span>{{ $t('approvals.viewDetails') }}</span>
        </button>
      </div>
    </article>
  </div>

  <div class="approval-queue-mobile" role="list">
    <article
      v-for="record in records"
      :key="`mobile-${record.kind}-${record.order.id}`"
      role="listitem"
      tabindex="0"
      @click="emit('open', record)"
      @keydown.enter="emit('open', record)"
      @keydown.space.prevent="emit('open', record)"
    >
      <header>
        <strong>{{ orderLabel(record) }} #{{ record.order.id }}</strong
        ><span class="approval-status">{{ $t('approvals.pending') }}</span
        ><button
          class="secondary-button approval-detail-button"
          type="button"
          :title="detailButtonTitle(record)"
          :aria-label="detailButtonAriaLabel(record)"
          @click.stop="emit('open', record)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="8" />
            <path d="M12 11v5M12 8h.01" />
          </svg>
          <span>{{ $t('approvals.viewDetails') }}</span>
        </button>
      </header>
      <p>{{ approvalContext(record) }}</p>
      <time :datetime="record.order.created_at"
        >{{ formatDate(record.order.created_at) }} ·
        {{ waitingLabel(record.order.created_at) }}</time
      >
      <div class="approval-queue-item">
        <AuthenticatedImage
          :file-id="firstItem(record).item_image_file_id"
          :alt="$t('approvals.itemMainImage', { name: firstItem(record).item_name })"
          :size="38"
          previewable
          @click.stop
          @keydown.stop
        />
        <div>
          <strong>{{ firstItem(record).item_name }}</strong
          ><small>{{ $t('approvals.itemLines', { n: record.order.items.length }) }} · {{ ruleSummary(record) }}</small>
        </div>
      </div>
    </article>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import {
  approvalContext,
  type ApprovalCatalog,
  type ApprovalRecord,
} from "../../pages/approvals/catalog";

defineProps<{ records: ApprovalRecord[]; catalog: ApprovalCatalog }>();
const emit = defineEmits<{ open: [record: ApprovalRecord] }>();
const { t } = useI18n();

function orderLabel(record: ApprovalRecord): string {
  return record.kind === "inbound" ? t("approvals.inboundOrder") : t("approvals.outboundOrder");
}
function detailButtonTitle(record: ApprovalRecord): string {
  return t("approvals.viewApprovalDetail", {
    kind: t(record.kind === "inbound" ? "approvals.inbound" : "approvals.outbound"),
  });
}
function detailButtonAriaLabel(record: ApprovalRecord): string {
  return t("approvals.viewDetailAria", {
    title: detailButtonTitle(record),
    order: orderLabel(record),
    n: record.order.id,
  });
}
function firstItem(record: ApprovalRecord) {
  return record.order.items[0];
}
function itemSummary(record: ApprovalRecord): string {
  const item = firstItem(record);
  return `${item.quantity} ${item.item_unit}`;
}
function ruleSummary(record: ApprovalRecord): string {
  if (record.kind === "inbound")
    return record.order.items.length === 1 ? record.order.items[0].location_name : t("approvals.writePerLine");
  return record.order.items.some((item) => item.batch_id !== null)
    ? t("approvals.withSpecifiedBatch")
    : t("approvals.fifoAtApproval");
}
function waitingLabel(value: string): string {
  const elapsed = Date.now() - new Date(value).getTime();
  if (!Number.isFinite(elapsed) || elapsed < 60_000) return t("approvals.justSubmitted");
  if (elapsed < 3_600_000) return t("approvals.waitMinutes", { n: Math.floor(elapsed / 60_000) });
  if (elapsed < 86_400_000) return t("approvals.waitHours", { n: Math.floor(elapsed / 3_600_000) });
  return t("approvals.waitDays", { n: Math.floor(elapsed / 86_400_000) });
}
function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(date);
}
</script>
