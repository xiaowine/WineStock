<!--
  本文件拥有前端根路由出口、服务断连覆盖层、本地服务恢复提示、路由切换进度条和全局 Notice 挂载点，属于 frontend。
  它不拥有具体页面布局、服务探测调度、平台 WebView 生命周期或 Axum 资源服务。
-->
<template>
  <div v-if="!viewportReady" class="app-viewport-bootstrap" aria-hidden="true" />
  <template v-else>
    <RouterView v-if="canRenderRoutes" />
    <Transition name="service-recovery">
      <div v-if="showRecoveryBanner" class="app-service-recovery" role="status" aria-live="polite">
        <span class="app-service-recovery__spinner" aria-hidden="true"></span>
        本地服务恢复中…
      </div>
    </Transition>
    <ServiceUnavailableScreen
      v-if="showServiceUnavailableScreen"
      :initial-check="showStableInitialCheck && !serviceBlocked"
      :busy="serviceScreenBusy"
      :variant="serviceScreenVariant"
      :error-message="shellServiceErrorMessage"
      @retry="handleServiceRetry"
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
import ServiceUnavailableScreen, {
  type ServiceUnavailableVariant,
} from "./components/ServiceUnavailableScreen.vue";
import { useStablePendingIndicator } from "./composables/useStablePendingIndicator";
import {
  checkServiceAvailability,
  isCheckingServiceAvailability,
  serviceAvailabilityStatus,
} from "./service/availability";
import { restartLocalService, runtimeSnapshot, startLocalService } from "./shell/runtime";

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
const serviceStopped = computed(() => serviceAvailabilityStatus.value === "stopped");
const serviceBlocked = computed(() => serviceUnavailable.value || serviceStopped.value);
const showServiceUnavailableScreen = computed(
  () =>
    route.meta.requiresService !== false && (serviceBlocked.value || showStableInitialCheck.value),
);

// 本地服务故障期间 Shell 自动重启；覆盖层保持不出现，只给轻量恢复提示。
const isRecovering = computed(() => serviceAvailabilityStatus.value === "recovering");
const showRecoveryBanner = useStablePendingIndicator(isRecovering, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});

const shellCapabilities = computed(() => runtimeSnapshot.value?.capabilities);
const localOwnership = computed(() => runtimeSnapshot.value?.service.ownership === "local");
const serviceScreenVariant = computed<ServiceUnavailableVariant>(() => {
  if (serviceStopped.value && shellCapabilities.value?.startLocalService) {
    return "local-stopped";
  }
  if (localOwnership.value && shellCapabilities.value?.restartLocalService) {
    return "local-failed";
  }
  return "remote";
});
const shellServiceErrorMessage = computed(() =>
  serviceScreenVariant.value === "remote"
    ? ""
    : (runtimeSnapshot.value?.service.error?.message ?? ""),
);

const serviceActionPending = ref(false);
const serviceScreenBusy = computed(
  () => isCheckingServiceAvailability.value || serviceActionPending.value,
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

/** 按语义变体分发重试动作；平台调用失败时快照会携带稳定错误，覆盖层继续呈现。 */
async function handleServiceRetry(): Promise<void> {
  if (serviceActionPending.value) {
    return;
  }
  const variant = serviceScreenVariant.value;
  if (variant === "remote") {
    void checkServiceAvailability();
    return;
  }
  serviceActionPending.value = true;
  try {
    if (variant === "local-stopped") {
      await startLocalService();
    } else {
      await restartLocalService();
    }
  } catch {
    // Shell 已通过快照发布失败详情；这里只需保持覆盖层等待下一次操作。
  } finally {
    serviceActionPending.value = false;
  }
}

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

/* 本地服务恢复中的轻量提示：不阻断页面，也不销毁当前上下文。 */
.app-service-recovery {
  position: fixed;
  z-index: var(--z-service-status);
  top: calc(12px + var(--safe-area-top));
  left: 50%;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid var(--color-border);
  border-radius: 999px;
  background: var(--color-surface);
  box-shadow: var(--shadow-service-status);
  color: var(--color-text);
  font-size: 13px;
  transform: translateX(-50%);

  &__spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--color-border-strong);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: app-service-recovery-spin 0.8s linear infinite;
  }
}

.service-recovery-enter-active,
.service-recovery-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.service-recovery-enter-from,
.service-recovery-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-6px);
}

@keyframes app-service-recovery-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
