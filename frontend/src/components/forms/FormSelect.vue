<!-- 本组件组合通用字段外壳和原生 select，选项内容仍由业务组件通过插槽提供。 -->
<template>
  <FormField
    :class="attrs.class"
    :style="attrs.style"
    :label="label"
    :control-id="selectId"
    :validation-key="validationKey"
    :error="error"
    :hint="hint"
    :required="required"
    v-slot="{ describedBy, invalid }"
  >
    <select
      :id="selectId"
      v-model="model"
      v-bind="controlAttrs"
      :required="required"
      :aria-invalid="invalid || undefined"
      :aria-describedby="mergeDescribedBy(controlAttrs['aria-describedby'], describedBy)"
    >
      <slot />
    </select>
  </FormField>
</template>

<script setup lang="ts">
import { computed, useAttrs, useId } from 'vue'
import FormField from './FormField.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  label: string
  validationKey?: string
  error?: string
  hint?: string
  required?: boolean
  id?: string
}>(), {
  validationKey: '',
  error: '',
  hint: '',
  required: false,
  id: undefined,
})

const model = defineModel<string | number | boolean | null | undefined>()
const attrs = useAttrs()
const uid = useId()
const selectId = computed(() => props.id ?? `form-select-${uid}`)
const controlAttrs = computed(() => Object.fromEntries(
  Object.entries(attrs).filter(([key]) => !['class', 'style', 'id'].includes(key)),
))

function mergeDescribedBy(current: unknown, generated: string | undefined): string | undefined {
  return [typeof current === 'string' ? current : '', generated ?? ''].filter(Boolean).join(' ') || undefined
}
</script>
