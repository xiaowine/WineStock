<!-- 本组件拥有审计日志高级筛选草稿和本地校验；它不请求事件 API。 -->
<template>
  <ModalDialog
    :open="open"
    title="更多审计筛选"
    description="按业务实体、操作人或分页数量进一步缩小结果范围。"
    @close="emit('close')"
  >
    <form id="event-filter-form" class="event-filter-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="entityId"
        label="实体 ID"
        type="number"
        min="1"
        step="1"
        placeholder="例如 42"
        :error="errors.entityId"
      />
      <FormInput
        v-model="userId"
        label="操作人用户 ID"
        type="number"
        min="1"
        step="1"
        placeholder="例如 1"
        :error="errors.userId"
      />
      <DateTimeField
        v-model="dateFrom"
        label="开始时间"
        :error="errors.dateRange"
      />
      <DateTimeField
        v-model="dateTo"
        label="结束时间"
        :error="errors.dateRange"
      />
      <FormInput
        v-model="customEntityType"
        label="实体类型原始值"
        maxlength="64"
        placeholder="可选，例如 custom_event"
        hint="填写后覆盖工具栏中的实体类型"
        :error="errors.customEntityType"
      />
      <FormInput
        v-model="customAction"
        label="动作原始值"
        maxlength="64"
        placeholder="可选，例如 archived"
        hint="填写后覆盖工具栏中的动作"
        :error="errors.customAction"
      />
      <label class="event-filter-form__page-size">
        <span>每页数量</span>
        <SelectControl v-model="pageSize" name="event_page_size">
          <option :value="25">25 条</option>
          <option :value="50">50 条</option>
          <option :value="100">100 条</option>
          <option :value="200">200 条</option>
        </SelectControl>
      </label>
    </form>

    <template #actions>
      <button class="text-button event-filter-form__reset" type="button" @click="reset">重置</button>
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button class="primary-button" type="submit" form="event-filter-form">应用筛选</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import DateTimeField from '../forms/DateTimeField.vue'
import FormInput from '../forms/FormInput.vue'
import SelectControl from '../forms/SelectControl.vue'
import ModalDialog from '../ModalDialog.vue'

export interface EventAdvancedFilterValue {
  entityId: number | null
  userId: number | null
  customEntityType: string
  customAction: string
  dateFrom: string
  dateTo: string
  pageSize: number
}

const props = defineProps<{
  open: boolean
  value: EventAdvancedFilterValue
}>()

const emit = defineEmits<{
  close: []
  apply: [value: EventAdvancedFilterValue]
}>()

const entityId = ref<number | null>(null)
const userId = ref<number | null>(null)
const customEntityType = ref('')
const customAction = ref('')
const dateFrom = ref('')
const dateTo = ref('')
const pageSize = ref(50)
const errors = reactive({ entityId: '', userId: '', customEntityType: '', customAction: '', dateRange: '' })

watch(
  () => props.open,
  (open) => {
    if (!open) return
    entityId.value = props.value.entityId
    userId.value = props.value.userId
    customEntityType.value = props.value.customEntityType
    customAction.value = props.value.customAction
    dateFrom.value = props.value.dateFrom
    dateTo.value = props.value.dateTo
    pageSize.value = props.value.pageSize
    clearErrors()
  },
  { immediate: true },
)

function reset(): void {
  entityId.value = null
  userId.value = null
  customEntityType.value = ''
  customAction.value = ''
  dateFrom.value = ''
  dateTo.value = ''
  pageSize.value = 50
  clearErrors()
}

function submit(): void {
  clearErrors()
  if (!validPositiveInteger(entityId.value)) errors.entityId = '实体 ID 必须是正整数'
  if (!validPositiveInteger(userId.value)) errors.userId = '用户 ID 必须是正整数'
  if (customEntityType.value && !customEntityType.value.trim()) errors.customEntityType = '实体类型不能只包含空格'
  if (customAction.value && !customAction.value.trim()) errors.customAction = '动作不能只包含空格'
  if (dateFrom.value && dateTo.value && new Date(dateFrom.value).getTime() > new Date(dateTo.value).getTime()) {
    errors.dateRange = '开始时间不能晚于结束时间'
  }
  if (Object.values(errors).some(Boolean)) return
  emit('apply', {
    entityId: entityId.value,
    userId: userId.value,
    customEntityType: customEntityType.value.trim(),
    customAction: customAction.value.trim(),
    dateFrom: dateFrom.value,
    dateTo: dateTo.value,
    pageSize: pageSize.value,
  })
}

function validPositiveInteger(value: number | null): boolean {
  return value === null || (Number.isInteger(value) && value > 0)
}

function clearErrors(): void {
  errors.entityId = ''
  errors.userId = ''
  errors.customEntityType = ''
  errors.customAction = ''
  errors.dateRange = ''
}
</script>

<style lang="scss" src="./EventFilterDialog.scss"></style>
