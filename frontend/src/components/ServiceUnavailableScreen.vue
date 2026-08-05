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
          运行模式
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
      title: "正在连接",
      body: "正在检查服务状态，请稍候。",
      retryLabel: "重新连接",
      retryBusyLabel: "正在连接…",
    };
  }
  switch (props.variant) {
    case "local-failed":
      return {
        title: "本地服务异常",
        body: "本地服务自动恢复失败。可以重试启动，或前往运行模式检查配置。",
        retryLabel: "重试启动",
        retryBusyLabel: "正在启动…",
      };
    case "local-stopped":
      return {
        title: "本地服务已停止",
        body: "本地服务当前处于停止状态，启动后将返回当前页面。",
        retryLabel: "启动服务",
        retryBusyLabel: "正在启动…",
      };
    default:
      return {
        title: "暂时无法连接服务",
        body: "请确认服务已启动且网络连接正常。系统会自动重试，连接恢复后将返回当前页面。",
        retryLabel: "重新连接",
        retryBusyLabel: "正在连接…",
      };
  }
});
</script>

<style scoped lang="scss" src="./ServiceUnavailableScreen.scss"></style>
