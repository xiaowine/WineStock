<!-- 本组件拥有入库单筛选草稿和日期范围校验；它不请求接口或修改路由。 -->
<template>
  <ModalDialog :open="open" title="筛选入库单" description="按处理状态和创建日期缩小入库单范围。" @close="emit('close')">
    <form id="inbound-order-filter-form" class="inbound-order-filter-form" novalidate @submit.prevent="submit">
      <FormSelect v-model="status" label="处理状态">
        <option value="">全部状态</option><option value="pending">待审批</option><option value="approved">已入库</option><option value="rejected">已拒绝</option>
      </FormSelect>
      <DateTimeField v-model="dateFrom" label="开始时间" :error="dateRangeError" />
      <DateTimeField v-model="dateTo" label="结束时间" :error="dateRangeError" />
    </form>
    <template #actions><button class="text-button inbound-order-filter-form__reset" type="button" @click="reset">重置</button><button class="secondary-button" type="button" @click="emit('close')">取消</button><button class="primary-button" type="submit" form="inbound-order-filter-form">应用筛选</button></template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { InboundOrderStatus } from '../../api/inboundOrders'
import DateTimeField from '../forms/DateTimeField.vue'
import FormSelect from '../forms/FormSelect.vue'
import ModalDialog from '../ModalDialog.vue'

export interface InboundOrderFilterValue { status: InboundOrderStatus | ''; dateFrom: string; dateTo: string }
const props = defineProps<{ open: boolean; value: InboundOrderFilterValue }>()
const emit = defineEmits<{ close: []; apply: [value: InboundOrderFilterValue] }>()
const status = ref<InboundOrderStatus | ''>(''); const dateFrom = ref(''); const dateTo = ref(''); const dateRangeError = ref('')
watch(() => props.open, (open) => { if (!open) return; status.value = props.value.status; dateFrom.value = props.value.dateFrom; dateTo.value = props.value.dateTo; dateRangeError.value = '' }, { immediate: true })
function reset(): void { status.value = ''; dateFrom.value = ''; dateTo.value = ''; dateRangeError.value = '' }
function submit(): void { dateRangeError.value = dateFrom.value && dateTo.value && dateFrom.value > dateTo.value ? '开始日期不能晚于结束日期' : ''; if (!dateRangeError.value) emit('apply', { status: status.value, dateFrom: dateFrom.value, dateTo: dateTo.value }) }
</script>

<style lang="scss" src="./InboundOrderFiltersDialog.scss"></style>
