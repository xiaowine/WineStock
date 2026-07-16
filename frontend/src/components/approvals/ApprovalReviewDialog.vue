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
      正在加载完整单据…
    </section>
    <section v-else-if="detailError" class="approval-detail-state approval-detail-state--error">
      <strong>无法加载审核资料</strong><span>{{ detailError }}</span
      ><button class="secondary-button" type="button" @click="emit('reload')">重试</button>
    </section>
    <template v-else-if="record">
      <section
        v-if="confirmAction"
        class="approval-confirmation"
        :class="`approval-confirmation--${confirmAction}`"
        role="status"
      >
        <strong>{{
          confirmAction === 'approve' ? '确认通过这张单据？' : '确认拒绝这张单据？'
        }}</strong>
        <p>
          {{ confirmAction === 'approve' ? catalog.approveConsequence : catalog.rejectConsequence }}
        </p>
        <span>{{ title }}</span>
      </section>
      <template v-else>
        <div class="approval-review">
          <section class="approval-impact">
            <header>
              <span class="approval-status">{{ statusLabel }}</span>
              <strong>{{
                record.kind === 'inbound' ? '通过后将增加库存' : '通过后将扣减库存'
              }}</strong>
              <time :datetime="record.order.created_at"
                >提交于 {{ formatDate(record.order.created_at) }}</time
              >
            </header>
            <p>{{ catalog.approveConsequence }}</p>
          </section>

          <section class="approval-review__section">
            <h3>单据信息</h3>
            <dl class="approval-detail-summary">
              <div>
                <dt>{{ catalog.contextLabel }}</dt>
                <dd>{{ approvalContext(record) }}</dd>
              </div>
              <div>
                <dt>创建时间</dt>
                <dd>{{ formatDate(record.order.created_at) }}</dd>
              </div>
              <div>
                <dt>创建人</dt>
                <dd>
                  {{
                    record.order.created_by_user_id
                      ? `用户 #${record.order.created_by_user_id}`
                      : '系统或未知用户'
                  }}
                </dd>
              </div>
              <div>
                <dt>备注</dt>
                <dd>{{ record.order.notes || '暂无备注' }}</dd>
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
          返回检查
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
          {{ actionBusy ? '正在处理…' : confirmAction === 'approve' ? '确认通过' : '确认拒绝' }}
        </button>
      </template>
      <template v-else>
        <button class="secondary-button" type="button" :disabled="actionBusy" @click="requestClose">
          关闭
        </button>
        <button
          class="secondary-button approval-reject-button"
          type="button"
          :disabled="!actionsEnabled"
          @click="confirmAction = 'reject'"
        >
          拒绝
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="!actionsEnabled"
          @click="confirmAction = 'approve'"
        >
          通过
        </button>
      </template>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { InboundOrderResponse } from '../../api/inboundOrders'
import type { OutboundOrderResponse } from '../../api/outboundOrders'
import {
  approvalContext,
  approvalId,
  type ApprovalCatalog,
  type ApprovalRecord,
} from '../../pages/approvals/catalog'
import ModalDialog from '../ModalDialog.vue'
import InboundApprovalDetails from './InboundApprovalDetails.vue'
import OutboundApprovalDetails from './OutboundApprovalDetails.vue'

const props = defineProps<{
  open: boolean
  record: ApprovalRecord | null
  catalog: ApprovalCatalog
  detailLoading: boolean
  detailError: string
  actionBusy: boolean
  actionError: string
}>()
const emit = defineEmits<{
  close: []
  reload: []
  act: [action: 'approve' | 'reject']
}>()
const confirmAction = ref<'approve' | 'reject' | null>(null)
const title = computed(() =>
  props.record
    ? `${props.record.kind === 'inbound' ? '入库单' : '出库单'} #${approvalId(props.record)}`
    : '审核单据',
)
const description = computed(() =>
  props.record?.kind === 'inbound'
    ? '核对待审批入库单的库存影响与业务明细。'
    : '核对待审批出库单的库存影响与业务明细。',
)
const inboundOrder = computed<InboundOrderResponse | null>(() =>
  props.record?.kind === 'inbound' ? props.record.order : null,
)
const outboundOrder = computed<OutboundOrderResponse | null>(() =>
  props.record?.kind === 'outbound' ? props.record.order : null,
)
const actionsEnabled = computed(() =>
  Boolean(
    props.record &&
    props.record.order.status === 'pending' &&
    !props.detailLoading &&
    !props.detailError &&
    !props.actionBusy,
  ),
)
const statusLabel = computed(() =>
  props.record?.order.status === 'pending'
    ? '待审批'
    : props.record?.order.status === 'approved'
      ? '已处理'
      : '已拒绝',
)

watch(
  () => [props.open, props.record ? approvalId(props.record) : null],
  () => {
    confirmAction.value = null
  },
)
function requestClose(): void {
  if (props.actionBusy) return
  if (confirmAction.value) {
    confirmAction.value = null
    return
  }
  emit('close')
}
function formatDate(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat('zh-CN', {
        dateStyle: 'medium',
        timeStyle: 'short',
      }).format(date)
}
</script>
