<!--
  本文件拥有已登录应用区域共用的嵌套路由出口和页面切换动效，属于 frontend 通用导航组件。
  它不决定路由权限、页面内容或桌面与移动 Shell 的布局。
-->
<template>
  <RouterView v-slot="{ Component, route }">
    <Transition name="route-page" mode="out-in">
      <component :is="Component" :key="route.fullPath" />
    </Transition>
  </RouterView>
</template>

<style lang="scss" src="./RouteContentView.scss"></style>
