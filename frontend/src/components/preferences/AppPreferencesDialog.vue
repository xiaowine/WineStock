<!--
  本组件拥有账户弹层入口的「偏好设置」Dialog：目前承载匿名数据收集开关，
  后续语言/主题等本机偏好也归于此处。改动即时生效并持久化；
  它不拥有偏好的存储格式（见 telemetry/consent.ts）与采集 SDK 生命周期细节（见 telemetry/clarity.ts）。
-->
<template>
  <ModalDialog
    :open="open"
    title="偏好设置"
    description="只影响这台设备上的使用体验，更改立即保存。"
    @close="emit('close')"
  >
    <section class="app-preferences__section" aria-label="数据收集">
      <h3>数据收集</h3>
      <label class="consent-toggle">
        <input
          v-model="telemetryEnabled"
          type="checkbox"
          name="preferences-telemetry"
          @change="handleTelemetryChange"
        />
        <span class="consent-toggle__copy">
          <strong>发送匿名使用数据</strong>
          <small>帮助开发者定位和排查问题；不包含库存内容与账户信息，仅在联网时生效。</small>
        </span>
      </label>
    </section>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import { startTelemetryIfConsented, stopTelemetry } from "../../telemetry/clarity";
import { readTelemetryConsent, saveTelemetryConsent } from "../../telemetry/consent";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const telemetryEnabled = ref(false);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    telemetryEnabled.value = readTelemetryConsent() === true;
  },
);

function handleTelemetryChange(): void {
  saveTelemetryConsent(telemetryEnabled.value);
  if (telemetryEnabled.value) {
    // 本会话内停过的采集会静默留待下次启动恢复，偏好本身已即时保存。
    startTelemetryIfConsented();
    return;
  }
  stopTelemetry();
}
</script>

<style scoped lang="scss">
/* 同意开关卡片复用 shared/_consent-toggle.scss；这里只保留分节标题。 */
.app-preferences__section {
  display: grid;
  gap: 10px;

  h3 {
    margin: 0;
    font-size: 13px;
  }
}
</style>
