<!--
  本文件拥有尚未实现业务页面的统一占位内容，属于 frontend 页面层。
  它只说明路由与后端 API 范围已经对齐，不请求接口或展示虚构业务数据。
-->
<template>
  <section class="route-page placeholder-page">
    <header class="content-header">
      <div>
        <h1>{{ title }}</h1>
        <p>{{ description }}</p>
      </div>
    </header>

    <section class="placeholder-page__state" aria-label="页面建设状态">
      <span>页面路由已接入</span>
      <h2>业务内容待实现</h2>
      <p>对应后端接口：{{ apiArea }}</p>
    </section>
  </section>
</template>

<script setup lang="ts">
defineProps<{
  /** 页面主标题，与路由元数据保持一致。 */
  title: string
  /** 当前页面未来承载的业务职责。 */
  description: string
  /** 从实时 OpenAPI 归纳出的接口范围。 */
  apiArea: string
}>()
</script>

<style scoped lang="scss">
.placeholder-page {
  gap: 24px;
}

.placeholder-page__state {
  display: grid;
  min-height: 240px;
  place-content: center;
  gap: 8px;
  padding: 28px;
  border: 1px dashed var(--color-border-strong);
  border-radius: var(--radius-lg);
  background: var(--color-surface-raised);
  text-align: center;
}

.placeholder-page__state span {
  color: var(--color-accent-strong);
  font-size: 13px;
  font-weight: 700;
}

.placeholder-page__state h2,
.placeholder-page__state p {
  margin: 0;
}

.placeholder-page__state h2 {
  font-size: 22px;
}

.placeholder-page__state p {
  max-width: 620px;
  color: var(--color-muted);
  line-height: 1.7;
}
</style>
