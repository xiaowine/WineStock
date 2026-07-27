<!-- 本组件拥有审批队列的桌面三段式和移动单列呈现；它不请求数据或执行审批。 -->
<template>
  <div class="approval-queue-table" role="table" aria-label="待审批单据">
    <div class="approval-queue-table__head" role="row">
      <span>单据与{{ catalog.contextLabel }}</span
      ><span>物品与库存规则</span><span>等待与操作</span>
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
            :alt="`${firstItem(record).item_name} 主图`"
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
        <span>{{ record.order.items.length }} 条明细 · {{ ruleSummary(record) }}</span>
      </div>
      <div class="approval-queue-table__decision" role="cell">
        <span class="approval-status">待审批</span
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
          <span>查看详情</span>
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
        ><span class="approval-status">待审批</span
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
          <span>查看详情</span>
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
          :alt="`${firstItem(record).item_name} 主图`"
          :size="38"
          previewable
          @click.stop
          @keydown.stop
        />
        <div>
          <strong>{{ firstItem(record).item_name }}</strong
          ><small>{{ record.order.items.length }} 条明细 · {{ ruleSummary(record) }}</small>
        </div>
      </div>
    </article>
  </div>
</template>

<script setup lang="ts">
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import {
  approvalContext,
  type ApprovalCatalog,
  type ApprovalRecord,
} from "../../pages/approvals/catalog";

defineProps<{ records: ApprovalRecord[]; catalog: ApprovalCatalog }>();
const emit = defineEmits<{ open: [record: ApprovalRecord] }>();

function orderLabel(record: ApprovalRecord): string {
  return record.kind === "inbound" ? "入库单" : "出库单";
}
function detailButtonTitle(record: ApprovalRecord): string {
  return `查看${record.kind === "inbound" ? "入库" : "出库"}审批详情`;
}
function detailButtonAriaLabel(record: ApprovalRecord): string {
  return `${detailButtonTitle(record)}：${orderLabel(record)} #${record.order.id}`;
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
    return record.order.items.length === 1 ? record.order.items[0].location_name : "按明细写入库位";
  return record.order.items.some((item) => item.batch_id !== null) ? "含指定批次" : "审批时按 FIFO";
}
function waitingLabel(value: string): string {
  const elapsed = Date.now() - new Date(value).getTime();
  if (!Number.isFinite(elapsed) || elapsed < 60_000) return "刚刚提交";
  if (elapsed < 3_600_000) return `等待 ${Math.floor(elapsed / 60_000)} 分钟`;
  if (elapsed < 86_400_000) return `等待 ${Math.floor(elapsed / 3_600_000)} 小时`;
  return `等待 ${Math.floor(elapsed / 86_400_000)} 天`;
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
