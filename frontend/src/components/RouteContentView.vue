<!--
  本文件拥有已登录应用区域唯一的嵌套路由出口和页面切换动效，属于 frontend 通用导航组件。
  它不决定路由权限、页面内容或桌面与移动布局。
-->
<template>
  <RouterView v-slot="{ Component, route }">
    <Transition name="route-page" mode="out-in">
      <!-- 只按路由身份重建页面，查询参数变化保留页面实例和业务上下文。 -->
      <component :is="Component" :key="route.name ?? route.path" />
    </Transition>
  </RouterView>
</template>

<style lang="scss" src="./RouteContentView.scss"></style>
