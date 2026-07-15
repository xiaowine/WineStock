<!--
  本组件拥有库位分组创建和编辑草稿、本地校验与字段错误呈现。
  它不调用库位 API，也不自行计算层级循环或子树深度。
-->
<template>
  <ModalDialog
    :open="open"
    :title="group ? '编辑库位分组' : '新建库位分组'"
    :description="group ? '修改名称、上级分组和展示顺序。' : '分组用于组织库区、楼层或仓储区域。'"
    :busy="submitting"
    @close="emit('close')"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="name"
        label="分组名称"
        validation-key="name"
        :error="errors.name"
        name="location_group_name"
        maxlength="128"
        autocomplete="off"
        autofocus
        required
        :disabled="submitting"
      />

      <FormSelect
        v-model="parentId"
        label="上级分组"
        validation-key="parent_id"
        :error="errors.parent_id"
        hint="分组层级最多 10 层"
        name="location_group_parent"
        :disabled="submitting"
      >
        <option :value="null">根分组</option>
        <option v-for="option in parentOptions" :key="option.id" :value="option.id">
          {{ option.label }}
        </option>
      </FormSelect>

      <FormInput
        v-model="sortOrder"
        label="排序"
        validation-key="sort_order"
        :error="errors.sort_order"
        hint="数值越小越靠前"
        name="location_group_sort_order"
        type="number"
        step="1"
        :disabled="submitting"
      />

      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">取消</button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? '正在保存…' : group ? '保存分组' : '创建分组' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, useId, watch } from 'vue'
import type { LocationGroupResponse, LocationGroupUpdateRequest } from '../../api/locations'
import { useFormValidation } from '../../composables/useFormValidation'
import { notice } from '../../notices/notice'
import ModalDialog from '../ModalDialog.vue'
import FormInput from '../forms/FormInput.vue'
import FormSelect from '../forms/FormSelect.vue'
import type { LocationGroupOption } from './types'

const props = defineProps<{
  open: boolean
  group: LocationGroupResponse | null
  defaultParentId: number | null
  parentOptions: LocationGroupOption[]
  submitting: boolean
  errorMessage: string
  fieldErrors: Record<string, string>
}>()

const emit = defineEmits<{
  close: []
  submit: [request: LocationGroupUpdateRequest]
}>()

const formId = `location-group-form-${useId()}`
const name = ref('')
const parentId = ref<number | null>(null)
const sortOrder = ref<number | null>(0)
const errors = ref<Record<string, string>>({})
useFormValidation(errors)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    name.value = props.group?.name ?? ''
    parentId.value = props.group?.parent_id ?? props.defaultParentId
    sortOrder.value = props.group?.sort_order ?? 0
    errors.value = { ...props.fieldErrors }
  },
)

watch(
  () => props.fieldErrors,
  (fieldErrors) => {
    if (props.open) errors.value = { ...fieldErrors }
  },
  { deep: true },
)

function submit(): void {
  const nextErrors: Record<string, string> = {}
  const normalizedName = name.value.trim()
  if (!normalizedName) nextErrors.name = '请输入分组名称'
  if (!Number.isInteger(sortOrder.value ?? 0)) nextErrors.sort_order = '排序必须是整数'
  errors.value = nextErrors
  if (Object.keys(nextErrors).length > 0) {
    notice.warning('请检查分组信息', { detail: Object.values(nextErrors)[0] })
    return
  }
  emit('submit', {
    parent_id: parentId.value,
    name: normalizedName,
    sort_order: sortOrder.value ?? 0,
  })
}
</script>
