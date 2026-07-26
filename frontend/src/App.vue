<!--
  本文件拥有前端根路由出口、服务断连覆盖层、路由切换进度条和全局 Notice 挂载点，属于 frontend。
  它不拥有具体页面布局、服务探测调度、平台 WebView 生命周期或 Axum 资源服务。
-->
<template>
  <div v-if="!viewportReady" class="app-viewport-bootstrap" aria-hidden="true" />
  <template v-else>
    <RouterView v-if="canRenderRoutes" />
    <ServiceUnavailableScreen
      v-if="showServiceUnavailableScreen"
      :initial-check="showStableInitialCheck && !serviceUnavailable"
      :checking="isCheckingServiceAvailability"
      @retry="checkServiceAvailability"
      @settings="openRuntimeSettings"
    />
  </template>
  <RouteProgressBar />
  <NoticeViewport />
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { authStatus } from "./auth/session";
import { waitForStableViewport } from "./bootstrap/viewport";
import NoticeViewport from "./components/NoticeViewport.vue";
import RouteProgressBar from "./components/RouteProgressBar.vue";
import ServiceUnavailableScreen from "./components/ServiceUnavailableScreen.vue";
import { useStablePendingIndicator } from "./composables/useStablePendingIndicator";
import {
  checkServiceAvailability,
  isCheckingServiceAvailability,
  serviceAvailabilityStatus,
} from "./service/availability";

const route = useRoute();
const router = useRouter();

const isInitialServiceCheck = computed(() => serviceAvailabilityStatus.value === "checking");
const viewportReady = ref(false);
const showStableInitialCheck = useStablePendingIndicator(isInitialServiceCheck, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const serviceUnavailable = computed(
  () => serviceAvailabilityStatus.value === "unavailable" || authStatus.value === "unavailable",
);
const showServiceUnavailableScreen = computed(
  () =>
    route.meta.requiresService !== false &&
    (serviceUnavailable.value || showStableInitialCheck.value),
);

// 首次成功连接后保持路由树挂载；后续断连只显示覆盖层，不能销毁当前页面和未保存上下文。
const hasMountedRoutes = ref(false);
watch(
  () => serviceAvailabilityStatus.value === "available" && !showStableInitialCheck.value,
  (ready) => {
    if (ready) hasMountedRoutes.value = true;
  },
  { immediate: true },
);
const canRenderRoutes = computed(
  () => viewportReady.value && (route.meta.requiresService === false || hasMountedRoutes.value),
);

onMounted(() => {
  void waitForStableViewport().then(() => {
    viewportReady.value = true;
  });
});

function openRuntimeSettings(): void {
  void router.push({
    name: "runtime-settings",
    query: { returnTo: route.fullPath },
  });
}
</script>

<style scoped lang="scss">
/* 视口稳定前只显示中性背景，避免错误宽度下出现桌面首帧。 */
.app-viewport-bootstrap {
  position: fixed;
  z-index: var(--z-service-status);
  inset: 0;
  background: var(--color-page);
}
</style>
