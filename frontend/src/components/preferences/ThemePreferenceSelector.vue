<!-- 本组件拥有本机主题三态选择控件；主题状态解析、持久化和平台同步仍由 theme runtime 负责。 -->
<template>
  <div class="theme-selector" role="radiogroup" aria-label="主题">
    <label v-for="choice in themeChoices" :key="choice.value" class="theme-selector__option">
      <input
        type="radio"
        :name="inputName"
        :value="choice.value"
        :checked="themePreference === choice.value"
        @change="setThemePreference(choice.value)"
      />
      <span>{{ choice.label }}</span>
    </label>
  </div>
</template>

<script setup lang="ts">
import { useId } from "vue";
import { setThemePreference, themePreference } from "../../theme/runtime";
import type { ThemePreference } from "../../theme/model";

const inputName = `theme-preference-${useId()}`;
const themeChoices: readonly { value: ThemePreference; label: string }[] = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];
</script>

<style scoped lang="scss">
.theme-selector {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
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
