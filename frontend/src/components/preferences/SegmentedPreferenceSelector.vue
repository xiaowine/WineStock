<!--
  本组件是偏好设置中的分段单选控件（外观/语言等本机偏好共用同一视觉模式）。
  值变更只上抛事件；状态与持久化由调用方拥有，本组件不读取任何运行时。
-->
<template>
  <div class="segmented-selector" role="radiogroup" :aria-label="ariaLabel">
    <label v-for="choice in options" :key="choice.value" class="segmented-selector__option">
      <input
        type="radio"
        :name="inputName"
        :value="choice.value"
        :checked="modelValue === choice.value"
        @change="emit('change', choice.value)"
      />
      <span>{{ choice.label }}</span>
    </label>
  </div>
</template>

<script setup lang="ts">
import { useId } from "vue";

defineProps<{
  /** 单选选项；label 应为调用方已本地化的展示文本。 */
  options: readonly { value: string; label: string }[];
  /** 当前选中值；变更后由调用方更新并回传。 */
  modelValue: string;
  /** 单选组无障碍标签。 */
  ariaLabel: string;
}>();

const emit = defineEmits<{ (event: "change", value: string): void }>();
const inputName = `segmented-preference-${useId()}`;
</script>

<style scoped lang="scss">
.segmented-selector {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);

  &__option {
    position: relative;
    min-width: 0;
    cursor: pointer;

    & + & {
      border-left: 1px solid var(--color-border);
    }

    input {
      position: absolute;
      width: 1px;
      height: 1px;
      opacity: 0;
      pointer-events: none;

      &:checked + span {
        background: var(--color-accent-soft);
        color: var(--color-accent-strong);
      }

      &:focus-visible + span {
        outline: 3px solid var(--color-focus-ring);
        outline-offset: -3px;
      }
    }

    span {
      display: grid;
      min-height: 38px;
      place-items: center;
      padding: 0 8px;
      color: var(--color-muted);
      font-size: 13px;
      font-weight: 650;
    }
  }
}
</style>
