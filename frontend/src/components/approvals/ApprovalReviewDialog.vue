<!-- 本组件拥有单张库存单据的只读审核、确认阶段和动作互斥界面；实际请求与队列协调由工作台负责。 -->
<template>
  <ModalDialog
    :open="open"
    workspace
    :busy="actionBusy"
    :title="title"
    :description="description"
    @close="requestClose"
  >
    <section v-if="detailLoading" class="approval-detail-state" aria-busy="true">
      {{ $t('approvals.loadingFullOrder') }}
    </section>
    <section v-else-if="detailError" class="approval-detail-state approval-detail-state--error">
      <strong>{{ $t('approvals.loadReviewFailed') }}</strong><span>{{ detailError }}</span
      ><button class="secondary-button" type="button" @click="emit('reload')">{{ $t('common.retry') }}</button>
    </section>
    <template v-else-if="record">
      <section
        v-if="confirmAction"
        class="approval-confirmation"
        :class="`approval-confirmation--${confirmAction}`"
        role="status"
      >
        <strong>{{
          confirmAction === "approve" ? $t('approvals.confirmApprove') : $t('approvals.confirmReject')
        }}</strong>
        <p>
          {{ confirmAction === "approve" ? $t(catalog.approveConsequence) : $t(catalog.rejectConsequence) }}
        </p>
        <span>{{ title }}</span>
      </section>
      <template v-else>
        <div class="approval-review">
          <section class="approval-impact">
            <header>
              <span class="approval-status">{{ statusLabel }}</span>
              <strong>{{
                record.kind === "inbound" ? $t('approvals.stockWillIncrease') : $t('approvals.stockWillDecrease')
              }}</strong>
              <time :datetime="record.order.created_at"
                >{{ $t('approvals.submittedAt', { time: formatDate(record.order.created_at) }) }}</time
              >
            </header>
            <p>{{ $t(catalog.approveConsequence) }}</p>
          </section>

          <section class="approval-review__section">
            <h3>{{ $t('approvals.orderInfo') }}</h3>
            <dl class="approval-detail-summary">
              <div>
                <dt>{{ $t(catalog.contextLabel) }}</dt>
                <dd>{{ approvalContext(record) }}</dd>
              </div>
              <div>
                <dt>{{ $t('approvals.createdAt') }}</dt>
                <dd>{{ formatDate(record.order.created_at) }}</dd>
              </div>
              <div>
                <dt>{{ $t('approvals.createdBy') }}</dt>
                <dd>
                  {{
                    record.order.created_by_user_id
                      ? $t('approvals.userRef', { n: record.order.created_by_user_id })
                      : $t('approvals.systemOrUnknown')
                  }}
                </dd>
              </div>
              <div>
                <dt>{{ $t('common.remark') }}</dt>
                <dd>{{ record.order.notes || $t('approvals.noRemark') }}</dd>
              </div>
            </dl>
          </section>

          <InboundApprovalDetails v-if="inboundOrder" :order="inboundOrder" />
          <OutboundApprovalDetails v-else-if="outboundOrder" :order="outboundOrder" />
        </div>
      </template>
    </template>

    <template v-if="actionError" #notice
      ><p class="form-warning" role="alert">{{ actionError }}</p></template
    >
    <template #actions>
      <template v-if="confirmAction">
        <button
          class="secondary-button"
          type="button"
          :disabled="actionBusy"
          @click="confirmAction = null"
        >
          {{ $t('approvals.backToReview') }}
        </button>
        <button
          :class="
            confirmAction === 'approve'
              ? 'primary-button'
              : 'secondary-button approval-reject-button'
          "
          type="button"
          :disabled="actionBusy"
          @click="emit('act', confirmAction)"
        >
          {{
            actionBusy
              ? $t('approvals.processing')
              : confirmAction === "approve"
                ? $t('approvals.confirmApproveAction')
                : $t('approvals.confirmRejectAction')
          }}
        </button>
      </template>
      <template v-else>
        <button class="secondary-button" type="button" :disabled="actionBusy" @click="requestClose">
          {{ $t('common.close') }}
        </button>
        <button
          class="secondary-button approval-reject-button"
          type="button"
          :disabled="!actionsEnabled"
          @click="confirmAction = 'reject'"
        >
          {{ $t('approvals.rejectAction') }}
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="!actionsEnabled"
          @click="confirmAction = 'approve'"
        >
          {{ $t('approvals.approveAction') }}
        </button>
      </template>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { InboundOrderResponse } from "../../api/inboundOrders";
import type { OutboundOrderResponse } from "../../api/outboundOrders";
import {
  approvalContext,
  approvalId,
  type ApprovalCatalog,
  type ApprovalRecord,
} from "../../pages/approvals/catalog";
import ModalDialog from "../ModalDialog.vue";
import InboundApprovalDetails from "./InboundApprovalDetails.vue";
import OutboundApprovalDetails from "./OutboundApprovalDetails.vue";

const props = defineProps<{
  open: boolean;
  record: ApprovalRecord | null;
  catalog: ApprovalCatalog;
  detailLoading: boolean;
  detailError: string;
  actionBusy: boolean;
  actionError: string;
}>();
const emit = defineEmits<{
  close: [];
  reload: [];
  act: [action: "approve" | "reject"];
}>();
const confirmAction = ref<"approve" | "reject" | null>(null);
const { t } = useI18n();
const title = computed(() =>
  props.record
    ? t("approvals.orderTitle", {
        label: t(
          props.record.kind === "inbound" ? "approvals.inboundOrder" : "approvals.outboundOrder",
        ),
        n: approvalId(props.record),
      })
    : t("approvals.reviewOrder"),
);
const description = computed(() =>
  props.record?.kind === "inbound"
    ? t("approvals.reviewInboundDescription")
    : t("approvals.reviewOutboundDescription"),
);
const inboundOrder = computed<InboundOrderResponse | null>(() =>
  props.record?.kind === "inbound" ? props.record.order : null,
);
const outboundOrder = computed<OutboundOrderResponse | null>(() =>
  props.record?.kind === "outbound" ? props.record.order : null,
);
const actionsEnabled = computed(() =>
  Boolean(
    props.record &&
    props.record.order.status === "pending" &&
    !props.detailLoading &&
    !props.detailError &&
    !props.actionBusy,
  ),
);
const statusLabel = computed(() =>
  props.record?.order.status === "pending"
    ? t("approvals.pending")
    : props.record?.order.status === "approved"
      ? t("approvals.processed")
      : t("approvals.rejected"),
);

watch(
  () => [props.open, props.record ? approvalId(props.record) : null],
  () => {
    confirmAction.value = null;
  },
);
function requestClose(): void {
  if (props.actionBusy) return;
  if (confirmAction.value) {
    confirmAction.value = null;
    return;
  }
  emit("close");
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
