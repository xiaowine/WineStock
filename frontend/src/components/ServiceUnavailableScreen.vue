<!--
  本文件拥有服务不可用/已停止时的全屏覆盖提示，属于 frontend 全局反馈层。
  它不执行 HTTP 请求、不管理鉴权会话，也不承载具体业务页面内容；
  文案按 ownership 语义分层（remote 断连 / 本地服务异常 / 本地服务已停止）。
-->
<template>
  <main
    class="service-unavailable"
    :role="assertive ? 'alert' : 'status'"
    :aria-live="assertive ? 'assertive' : 'polite'"
  >
    <section class="service-unavailable__panel" aria-labelledby="service-status-title">
      <div
        class="service-unavailable__indicator"
        :class="{ 'service-unavailable__indicator--calm': !assertive }"
        aria-hidden="true"
      >
        <span></span>
      </div>
      <div class="service-unavailable__content">
        <p class="service-unavailable__eyebrow">WineStock</p>
        <h1 id="service-status-title">{{ copy.title }}</h1>
        <p>{{ copy.body }}</p>
        <p v-if="!initialCheck && errorMessage" class="service-unavailable__error">
          {{ errorMessage }}
        </p>
      </div>
      <div v-show="!initialCheck" class="service-unavailable__actions">
        <button class="secondary-button" type="button" :disabled="busy" @click="$emit('settings')">
          {{ $t("startup.runtimeMode") }}
        </button>
        <button
          class="primary-button service-unavailable__retry"
          type="button"
          :disabled="busy"
          @click="$emit('retry')"
        >
          {{ busy ? copy.retryBusyLabel : copy.retryLabel }}
        </button>
      </div>
    </section>
  </main>
</template>

<script lang="ts">
/** 覆盖层的语义变体；决定标题、正文与主操作文案。 */
export type ServiceUnavailableVariant = "remote" | "local-failed" | "local-stopped";
</script>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    /** 是否仍处于应用启动后的首次服务探测。 */
    initialCheck: boolean;
    /** 是否正在执行健康检查或平台服务操作（禁用重复操作）。 */
    busy: boolean;
    /** 语义变体；默认维持远端断连语义。 */
    variant?: ServiceUnavailableVariant;
    /** Shell 报告的稳定错误信息（仅本地语义展示）。 */
    errorMessage?: string;
  }>(),
  { variant: "remote", errorMessage: "" },
);

defineEmits<{
  /** 用户要求重试；具体动作（重连/重启/启动）由宿主按平台能力决定。 */
  retry: [];
  /** 用户要求打开不依赖 API 的运行模式配置。 */
  settings: [];
}>();

/** 已停止是用户主动造成的中性状态，不用 alert 打断读屏。 */
const assertive = computed(() => !props.initialCheck && props.variant !== "local-stopped");

const copy = computed(() => {
  if (props.initialCheck) {
    return {
      title: t("startup.connectingTitle"),
      body: t("startup.connectingBody"),
      retryLabel: t("startup.reconnect"),
      retryBusyLabel: t("startup.connectingBusy"),
    };
  }
  switch (props.variant) {
    case "local-failed":
      return {
        title: t("startup.localFailedTitle"),
        body: t("startup.localFailedBody"),
        retryLabel: t("startup.restartService"),
        retryBusyLabel: t("startup.startingBusy"),
      };
    case "local-stopped":
      return {
        title: t("startup.localStoppedTitle"),
        body: t("startup.localStoppedBody"),
        retryLabel: t("startup.startService"),
        retryBusyLabel: t("startup.startingBusy"),
      };
    default:
      return {
        title: t("startup.remoteUnreachableTitle"),
        body: t("startup.remoteUnreachableBody"),
        retryLabel: t("startup.reconnect"),
        retryBusyLabel: t("startup.connectingBusy"),
      };
  }
});
</script>

<style scoped lang="scss" src="./ServiceUnavailableScreen.scss"></style>
