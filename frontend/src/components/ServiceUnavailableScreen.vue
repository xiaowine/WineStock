<!--
  本文件拥有服务断连时的全屏覆盖提示，属于 frontend 全局反馈层。
  它不执行 HTTP 请求、不管理鉴权会话，也不承载具体业务页面内容。
-->
<template>
  <main
    class="service-unavailable"
    :role="initialCheck ? 'status' : 'alert'"
    :aria-live="initialCheck ? 'polite' : 'assertive'"
  >
    <section class="service-unavailable__panel" aria-labelledby="service-status-title">
      <div class="service-unavailable__indicator" aria-hidden="true">
        <span></span>
      </div>
      <div class="service-unavailable__content">
        <p class="service-unavailable__eyebrow">WineStock</p>
        <h1 id="service-status-title">
          {{ initialCheck ? "正在连接" : "暂时无法连接服务" }}
        </h1>
        <p>
          {{
            initialCheck
              ? "正在检查服务状态，请稍候。"
              : "请确认服务已启动且网络连接正常。系统会自动重试，连接恢复后将返回当前页面。"
          }}
        </p>
      </div>
      <button
        v-show="!initialCheck"
        class="primary-button service-unavailable__retry"
        type="button"
        :disabled="checking"
        @click="$emit('retry')"
      >
        {{ checking ? "正在连接…" : "重新连接" }}
      </button>
    </section>
  </main>
</template>

<script setup lang="ts">
defineProps<{
  /** 是否仍处于应用启动后的首次服务探测。 */
  initialCheck: boolean;
  /** 是否正在执行健康检查。 */
  checking: boolean;
}>();

defineEmits<{
  /** 用户要求立即重新检查服务。 */
  retry: [];
}>();
</script>

<style scoped lang="scss" src="./ServiceUnavailableScreen.scss"></style>
