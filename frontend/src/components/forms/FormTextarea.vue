<!-- 本组件组合通用字段外壳和固定尺寸 textarea，统一错误呈现并禁止用户拖动改变布局。 -->
<template>
  <FormField
    :class="attrs.class"
    :style="attrs.style"
    :label="label"
    :control-id="textareaId"
    :validation-key="validationKey"
    :error="error"
    :hint="hint"
    :required="required"
    v-slot="{ describedBy, invalid }"
  >
    <textarea
      :id="textareaId"
      v-model="model"
      v-bind="controlAttrs"
      autocomplete="off"
      :required="required"
      :aria-invalid="invalid || undefined"
      :aria-describedby="mergeDescribedBy(controlAttrs['aria-describedby'], describedBy)"
    />
  </FormField>
</template>

<script setup lang="ts">
import { computed, useAttrs, useId } from "vue";
import FormField from "./FormField.vue";

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    label: string;
    validationKey?: string;
    error?: string;
    hint?: string;
    required?: boolean;
    id?: string;
  }>(),
  {
    validationKey: "",
    error: "",
    hint: "",
    required: false,
    id: undefined,
  },
);

const model = defineModel<string>({ required: true });
const attrs = useAttrs();
const uid = useId();
const textareaId = computed(() => props.id ?? `form-textarea-${uid}`);
const controlAttrs = computed(() =>
  Object.fromEntries(
    Object.entries(attrs).filter(([key]) => !["class", "style", "id"].includes(key)),
  ),
);

function mergeDescribedBy(current: unknown, generated: string | undefined): string | undefined {
  return (
    [typeof current === "string" ? current : "", generated ?? ""].filter(Boolean).join(" ") ||
    undefined
  );
}
</script>
