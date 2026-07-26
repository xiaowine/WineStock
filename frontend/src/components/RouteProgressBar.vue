<!--
  本文件拥有路由切换期间的全局顶部进度条呈现，属于 frontend 通用导航组件。
  它只消费稳定后的路由等待状态，不拥有计时策略或导航逻辑。
-->
<template>
  <Transition name="route-progress">
    <div v-if="routeNavigationIndicatorVisible" class="route-progress" role="status">
      <span class="route-progress__bar" aria-hidden="true" />
      <span class="visually-hidden">页面加载中</span>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { routeNavigationIndicatorVisible } from "../router/navigationPending";
</script>

<style scoped lang="scss">
/* 低于服务状态覆盖层：启动门期间由启动门自己的提示承担反馈。 */
.route-progress {
  position: fixed;
  top: var(--safe-area-top, 0px);
  right: 0;
  left: 0;
  z-index: var(--z-notice);
  height: 3px;
  overflow: hidden;
  background: var(--color-accent-soft);
  pointer-events: none;
}

.route-progress__bar {
  display: block;
  width: 40%;
  height: 100%;
  border-radius: 999px;
  background: var(--color-accent);
  animation: route-progress-slide 1.1s ease-in-out infinite;
}

@keyframes route-progress-slide {
  from {
    transform: translateX(-100%);
  }

  to {
    transform: translateX(250%);
  }
}

.route-progress-enter-active,
.route-progress-leave-active {
  transition: opacity var(--motion-duration-fast) var(--motion-ease-standard);
}

.route-progress-enter-from,
.route-progress-leave-to {
  opacity: 0;
}

/* 减少动态效果时只去掉滑动动画，等待状态仍按稳定计时呈现。 */
@media (prefers-reduced-motion: reduce) {
  .route-progress__bar {
    width: 100%;
    animation: none;
    opacity: 0.7;
  }
}
</style>
