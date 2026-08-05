<!-- 本组件拥有运行方式的三项选择呈现；它不校验配置、不保存草稿或启动本地服务。 -->
<template>
  <fieldset class="runtime-mode-selector" :disabled="disabled">
    <legend>运行模式</legend>
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
import type { RuntimeMode } from "../../shell/contract";

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
    label: "本机自用",
    description: "服务和数据运行在这台设备上，适合个人或单设备使用。",
    selected: props.modelValue === "self-hosted",
    disabled: false,
    unavailableReason: "",
  },
  {
    value: "client-only" as const,
    label: "连接远端",
    description: "不启动本地服务，连接已经部署好的 WineStock 服务。",
    selected: props.modelValue === "client-only" || props.modelValue === "connect-to-remote",
    disabled: false,
    unavailableReason: "",
  },
  {
    value: "server-mode" as const,
    label: "共享服务",
    description: "在这台设备运行服务，允许其他设备连接使用。",
    selected: props.modelValue === "server-mode",
    disabled: !props.serverModeAvailable,
    unavailableReason: props.serverModeAvailable
      ? ""
      : "当前平台暂不支持自动配置防火墙，请手动配置。",
  },
]);

function selectMode(mode: RuntimeMode): void {
  if (!props.disabled) {
    emit("update:modelValue", mode);
  }
}
</script>

<style lang="scss" src="./RuntimeModeSelector.scss"></style>
