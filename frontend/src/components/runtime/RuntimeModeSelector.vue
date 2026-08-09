<!-- 本组件拥有运行方式的三项选择呈现；它不校验配置、不保存草稿或启动本地服务。 -->
<template>
  <fieldset class="runtime-mode-selector" :disabled="disabled">
    <legend>{{ $t("runtime.runtimeMode") }}</legend>
    <label
      v-for="option in options"
      :key="option.value"
      class="runtime-mode-option"
      :class="{
        'runtime-mode-option--selected': option.selected,
        'runtime-mode-option--disabled': option.disabled,
      }"
    >
      <input
        type="radio"
        name="runtime_mode"
        :value="option.value"
        :checked="option.selected"
        :disabled="option.disabled"
        @change="selectMode(option.value)"
      />
      <span class="runtime-mode-option__indicator" aria-hidden="true"></span>
      <span class="runtime-mode-option__copy">
        <strong>{{ option.label }}</strong>
        <small>{{ option.description }}</small>
        <em v-if="option.unavailableReason">{{ option.unavailableReason }}</em>
      </span>
    </label>
  </fieldset>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { RuntimeMode } from "../../shell/contract";

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    modelValue: RuntimeMode;
    serverModeAvailable?: boolean;
    disabled?: boolean;
  }>(),
  {
    serverModeAvailable: true,
    disabled: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: RuntimeMode];
}>();

const options = computed(() => [
  {
    value: "self-hosted" as const,
    label: t("runtime.localModeLabel"),
    description: t("runtime.localModeDescription"),
    selected: props.modelValue === "self-hosted",
    disabled: false,
    unavailableReason: "",
  },
  {
    value: "client-only" as const,
    label: t("runtime.remoteModeLabel"),
    description: t("runtime.remoteModeDescription"),
    selected: props.modelValue === "client-only" || props.modelValue === "connect-to-remote",
    disabled: false,
    unavailableReason: "",
  },
  {
    value: "server-mode" as const,
    label: t("runtime.serverModeLabel"),
    description: t("runtime.serverModeDescription"),
    selected: props.modelValue === "server-mode",
    disabled: !props.serverModeAvailable,
    unavailableReason: props.serverModeAvailable ? "" : t("runtime.firewallManualConfig"),
  },
]);

function selectMode(mode: RuntimeMode): void {
  if (!props.disabled) {
    emit("update:modelValue", mode);
  }
}
</script>

<style lang="scss" src="./RuntimeModeSelector.scss"></style>
