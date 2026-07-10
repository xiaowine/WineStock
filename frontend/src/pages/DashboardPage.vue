<!--
  本文件拥有总览路由的页面骨架，属于 frontend 页面层。
  它暂不拥有看板 API 调用、业务指标定义或平台生命周期。
-->
<template>
  <section class="route-page">
    <header class="content-header">
      <div>
        <p class="eyebrow">Dashboard</p>
        <h1>总览</h1>
      </div>
    </header>

    <section class="route-placeholder" aria-labelledby="dashboard-placeholder-title">
      <p class="eyebrow">路由骨架</p>
      <h2 id="dashboard-placeholder-title">总览页面已接入应用壳</h2>
      <p>后续将在这里接入库存摘要与趋势接口；当前不保留原型中的假指标。</p>
      <RouterLink class="secondary-button" :to="{ name: 'items' }">前往物品页面</RouterLink>
    </section>
  </section>
</template>
