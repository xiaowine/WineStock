<!-- 本组件组合通用字段外壳和原生 input，统一错误状态并保持原生输入属性可透传。 -->
<template>
  <FormField
    :class="attrs.class"
    :style="attrs.style"
    :label="label"
    :control-id="inputId"
    :validation-key="validationKey"
    :error="error"
    :hint="hint"
    :required="required"
    v-slot="{ describedBy, invalid }"
  >
    <input
      v-bind="controlAttrs"
      :id="inputId"
      :type="type"
      :value="modelValue ?? ''"
      :required="required"
      :aria-invalid="invalid || undefined"
      :aria-describedby="mergeDescribedBy(controlAttrs['aria-describedby'], describedBy)"
      @input="updateValue"
    />
  </FormField>
</template>

<script setup lang="ts">
import { computed, useAttrs, useId } from 'vue'
import FormField from './FormField.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  modelValue?: string | number | null
  label: string
  validationKey?: string
  error?: string
  hint?: string
  required?: boolean
  type?: string
  id?: string
}>(), {
  modelValue: '',
  validationKey: '',
  error: '',
  hint: '',
  required: false,
  type: 'text',
  id: undefined,
})

const emit = defineEmits<{ 'update:modelValue': [value: string | number | null] }>()
const attrs = useAttrs()
const uid = useId()
const inputId = computed(() => props.id ?? `form-input-${uid}`)
const controlAttrs = computed(() => Object.fromEntries(
  Object.entries(attrs).filter(([key]) => key !== 'class' && key !== 'style'),
))

function updateValue(event: Event): void {
  const input = event.target as HTMLInputElement
  if (props.type === 'number') {
    emit('update:modelValue', input.value === '' ? null : input.valueAsNumber)
    return
  }
  emit('update:modelValue', input.value)
}

function mergeDescribedBy(current: unknown, generated: string | undefined): string | undefined {
  return [typeof current === 'string' ? current : '', generated ?? ''].filter(Boolean).join(' ') || undefined
}
</script>
