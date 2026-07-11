<!--
  本文件拥有前端根路由出口、服务断连阻断层和全局 Notice 挂载点，属于 frontend。
  它不拥有具体页面布局、服务探测调度、平台 WebView 生命周期或 Axum 资源服务。
-->
<template>
  <ServiceUnavailableScreen
    v-if="showServiceUnavailableScreen"
    :initial-check="showStableInitialCheck && !serviceUnavailable"
    :checking="isCheckingServiceAvailability"
    @retry="checkServiceAvailability"
  />
  <RouterView v-else-if="canRenderRoutes" />
  <NoticeViewport />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { authStatus } from './auth/session'
import NoticeViewport from './components/NoticeViewport.vue'
import ServiceUnavailableScreen from './components/ServiceUnavailableScreen.vue'
import { useStablePendingIndicator } from './composables/useStablePendingIndicator'
import {
  checkServiceAvailability,
  isCheckingServiceAvailability,
  serviceAvailabilityStatus,
} from './service/availability'

const isInitialServiceCheck = computed(() => serviceAvailabilityStatus.value === 'checking')
const showStableInitialCheck = useStablePendingIndicator(isInitialServiceCheck, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
})
const serviceUnavailable = computed(
  () =>
    serviceAvailabilityStatus.value === 'unavailable' || authStatus.value === 'unavailable',
)
const showServiceUnavailableScreen = computed(
  () => serviceUnavailable.value || showStableInitialCheck.value,
)
const canRenderRoutes = computed(
  () => serviceAvailabilityStatus.value === 'available' && !showStableInitialCheck.value,
)
</script>
