<!-- 本组件拥有分类创建与编辑草稿和本地校验，不调用分类 API。 -->
<template>
  <ModalDialog
    :open="open"
    :title="category ? '编辑物品分类' : '新建物品分类'"
    :description="category ? '修改分类名称、说明和展示顺序。' : '分类用于物品归类，不包含属性字段。'"
    :busy="submitting"
    @close="requestClose"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="name"
        label="分类名称"
        validation-key="name"
        :error="errors.name"
        maxlength="128"
        autocomplete="off"
        autofocus
        required
        :disabled="submitting"
      />
      <FormTextarea
        v-model="description"
        label="分类说明"
        validation-key="description"
        :error="errors.description"
        maxlength="1024"
        :rows="4"
        :disabled="submitting"
      />
      <FormInput
        v-model="sortOrder"
        label="排序"
        validation-key="sort_order"
        :error="errors.sort_order"
        hint="数值越小越靠前"
        type="number"
        min="0"
        step="1"
        :disabled="submitting"
      />
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="requestClose">取消</button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? '正在保存…' : category ? '保存分类' : '创建分类' }}
      </button>
    </template>
  </ModalDialog>

  <ModalDialog
    :open="discardOpen"
    title="放弃未保存修改？"
    description="关闭后，本次填写的分类信息不会保留。"
    compact
    nested
    @close="discardOpen = false"
  >
    <template #actions>
      <button class="secondary-button" type="button" @click="discardOpen = false">继续编辑</button>
      <button class="danger-button" type="button" @click="confirmClose">放弃修改</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue'
import type { ItemCategoryResponse, ItemCategoryWriteRequest } from '../../api/itemCategories'
import { useFormValidation } from '../../composables/useFormValidation'
import { notice } from '../../notices/notice'
import ModalDialog from '../ModalDialog.vue'
import FormInput from '../forms/FormInput.vue'
import FormTextarea from '../forms/FormTextarea.vue'

const props = defineProps<{
  open: boolean
  category: ItemCategoryResponse | null
  defaultSortOrder: number
  submitting: boolean
  errorMessage: string
  fieldErrors: Record<string, string>
}>()

const emit = defineEmits<{
  close: []
  submit: [request: ItemCategoryWriteRequest]
}>()

const formId = `category-form-${useId()}`
const name = ref('')
const description = ref('')
const sortOrder = ref<number | null>(0)
const errors = ref<Record<string, string>>({})
const initialSnapshot = ref('')
const discardOpen = ref(false)
useFormValidation(errors)

const snapshot = computed(() => JSON.stringify([name.value, description.value, sortOrder.value]))

watch(() => props.open, (open) => {
  if (!open) return
  name.value = props.category?.name ?? ''
  description.value = props.category?.description ?? ''
  sortOrder.value = props.category?.sort_order ?? props.defaultSortOrder
  errors.value = { ...props.fieldErrors }
  discardOpen.value = false
  initialSnapshot.value = JSON.stringify([name.value, description.value, sortOrder.value])
})

watch(() => props.fieldErrors, (fieldErrors) => {
  if (props.open) errors.value = { ...fieldErrors }
}, { deep: true })

function submit(): void {
  const nextErrors: Record<string, string> = {}
  const normalizedName = name.value.trim()
  const normalizedDescription = description.value.trim()
  if (!normalizedName) nextErrors.name = '请输入分类名称'
  else if (normalizedName.length > 128) nextErrors.name = '分类名称不能超过 128 个字符'
  if (normalizedDescription.length > 1024) nextErrors.description = '分类说明不能超过 1024 个字符'
  if (!Number.isInteger(sortOrder.value) || (sortOrder.value ?? -1) < 0) nextErrors.sort_order = '排序必须是大于等于零的整数'
  errors.value = nextErrors
  if (Object.keys(nextErrors).length) {
    notice.warning('请检查分类信息', { detail: Object.values(nextErrors)[0] })
    return
  }
  emit('submit', {
    name: normalizedName,
    description: normalizedDescription || null,
    sort_order: sortOrder.value as number,
  })
}

function requestClose(): void {
  if (snapshot.value !== initialSnapshot.value) discardOpen.value = true
  else emit('close')
}

function confirmClose(): void {
  discardOpen.value = false
  emit('close')
}
</script>
