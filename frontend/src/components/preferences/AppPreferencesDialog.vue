<!--
  本组件拥有账户弹层入口的「偏好设置」Dialog：承载主题和匿名数据收集等本机偏好。
  改动即时生效并持久化；它不拥有偏好存储格式、主题运行时或采集 SDK 生命周期细节。
-->
<template>
  <ModalDialog
    :open="open"
    title="偏好设置"
    description="只影响这台设备上的使用体验，更改立即保存。"
    @close="emit('close')"
  >
    <div class="app-preferences">
      <section class="app-preferences__section" aria-labelledby="preferences-appearance-title">
        <h3 id="preferences-appearance-title">外观</h3>
        <ThemePreferenceSelector />
      </section>

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
            <small
              >帮助开发者定位和排查问题；不包含库存内容与账户信息，仅在联网时生效。分析服务由
              Microsoft Clarity 提供。</small
            >
          </span>
        </label>
        <p class="app-preferences__policy">
          <a href="#" @click.prevent="openTelemetryPolicy">查看 Microsoft 隐私声明</a>
        </p>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">关闭</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import { startTelemetryIfConsented, stopTelemetry } from "../../telemetry/clarity";
import {
  TELEMETRY_POLICY_URL,
  readTelemetryConsent,
  saveTelemetryConsent,
} from "../../telemetry/consent";
import { openExternal } from "../../shell/runtime";
import ThemePreferenceSelector from "./ThemePreferenceSelector.vue";

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

/** 打开 Microsoft 隐私声明；平台不支持外链能力时静默忽略。 */
function openTelemetryPolicy(): void {
  void openExternal(TELEMETRY_POLICY_URL).catch(() => undefined);
}
</script>

<style scoped lang="scss">
/* 同意开关卡片复用 shared/_consent-toggle.scss；这里只保留偏好分节、主题分段控件与政策链接。 */
.app-preferences {
  display: grid;
  gap: 18px;
}

.app-preferences__section {
  display: grid;
  gap: 10px;

  & + & {
    padding-top: 18px;
    border-top: 1px solid var(--color-border);
  }

  h3 {
    margin: 0;
    font-size: 13px;
  }
}

.app-preferences__policy {
  margin: 0;
  font-size: 12px;

  a {
    color: var(--color-accent);
    font-weight: 650;
  }

  a:hover {
    color: var(--color-accent-strong);
    text-decoration: underline;
  }
}
</style>
