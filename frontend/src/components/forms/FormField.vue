<!-- 本组件统一字段标题、提示、红框错误状态、无障碍错误说明和校验定位；不包含业务校验规则。 -->
<template>
  <div
    ref="root"
    v-bind="attrs"
    class="form-field"
    @input="clearCurrentError"
    @change="clearCurrentError"
  >
    <label v-if="label && controlId" :for="controlId">{{ fieldLabel }}</label>
    <span v-else-if="label">{{ fieldLabel }}</span>
    <slot
      :control-id="controlId"
      :described-by="describedBy"
      :invalid="Boolean(error)"
    />
    <small v-if="hint" :id="hintId" class="field-hint">{{ hint }}</small>
    <small v-if="error" :id="errorId" class="visually-hidden" role="alert">{{ error }}</small>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useAttrs, useId, watch } from 'vue'
import { useFormValidationContext } from '../../composables/useFormValidation'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  label?: string
  controlId?: string
  validationKey?: string
  error?: string
  hint?: string
  required?: boolean
}>(), {
  label: '',
  controlId: undefined,
  validationKey: '',
  error: '',
  hint: '',
  required: false,
})

const attrs = useAttrs()
const root = ref<HTMLElement | null>(null)
const validation = useFormValidationContext()
const uid = useId()
const hintId = `form-field-${uid}-hint`
const errorId = `form-field-${uid}-error`
const fieldLabel = computed(() => props.required ? `${props.label} *` : props.label)
const describedBy = computed(() => [props.hint ? hintId : '', props.error ? errorId : ''].filter(Boolean).join(' ') || undefined)
let unregister: (() => void) | undefined

onMounted(registerCurrentField)
onBeforeUnmount(() => unregister?.())
watch(() => props.validationKey, registerCurrentField)

function registerCurrentField(): void {
  unregister?.()
  unregister = props.validationKey && validation
    ? validation.registerField(props.validationKey, () => root.value)
    : undefined
}

function clearCurrentError(): void {
  if (props.error && props.validationKey) validation?.clearFieldError(props.validationKey)
}
</script>
