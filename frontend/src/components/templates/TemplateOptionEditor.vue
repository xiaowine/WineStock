<!-- 本组件拥有模板候选项的增删和顺序调整，不解释候选项业务含义。 -->
<template>
  <fieldset class="template-option-editor" :disabled="disabled">
    <legend>{{ label }} <span>{{ model.length }} / {{ maxItems }}</span></legend>
    <div v-for="(_option, index) in model" :key="index" class="template-option-editor__row">
      <span class="template-option-editor__number">{{ index + 1 }}</span>
      <input
        v-model="model[index]"
        type="text"
        :maxlength="maxLength"
        :aria-label="`${label} ${index + 1}`"
        :aria-invalid="Boolean(errors[`${errorPrefix}.${index}`]) || undefined"
        autocomplete="off"
      />
      <button class="icon-button" type="button" title="上移" :disabled="disabled || index === 0" @click="move(index, -1)">↑</button>
      <button class="icon-button" type="button" title="下移" :disabled="disabled || index === model.length - 1" @click="move(index, 1)">↓</button>
      <button class="icon-button template-option-editor__delete" type="button" title="删除" :disabled="disabled" @click="model.splice(index, 1)">×</button>
      <span v-if="errors[`${errorPrefix}.${index}`]" class="template-option-editor__error">{{ errors[`${errorPrefix}.${index}`] }}</span>
    </div>
    <p v-if="errors[errorPrefix]" class="form-error">{{ errors[errorPrefix] }}</p>
    <button class="text-button" type="button" :disabled="disabled || model.length >= maxItems" @click="add">+ 添加候选项</button>
  </fieldset>
</template>

<script setup lang="ts">
import { nextTick } from 'vue'

const props = defineProps<{
  label: string
  errorPrefix: string
  errors: Record<string, string>
  maxItems: number
  maxLength: number
  disabled: boolean
}>()

const model = defineModel<string[]>({ required: true })

function add(): void {
  if (model.value.length >= props.maxItems) return
  model.value.push('')
  void nextTick(() => {
    const inputs = document.querySelectorAll<HTMLInputElement>('.template-option-editor input')
    inputs.item(inputs.length - 1)?.focus()
  })
}

function move(index: number, offset: -1 | 1): void {
  const target = index + offset
  if (target < 0 || target >= model.value.length) return
  const [option] = model.value.splice(index, 1)
  model.value.splice(target, 0, option)
}
</script>
