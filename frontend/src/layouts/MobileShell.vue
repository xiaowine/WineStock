<!--
  本文件拥有移动端应用壳，属于 frontend 布局层。
  它只组织顶部上下文栏、主容器和左侧导航抽屉，不拥有原生 Android 生命周期。
-->
<template>
  <div class="app-shell mobile-shell" data-shell="mobile">
    <header class="mobile-topbar">
      <button class="icon-button" type="button" aria-label="打开导航" @click="navOpen = true">☰</button>
      <div class="mobile-topbar__title">
        <span>库存总览</span>
        <strong>物料库存</strong>
      </div>
      <div class="mobile-topbar__actions">
        <button class="icon-button" type="button" aria-label="搜索">⌕</button>
        <button class="icon-button" type="button" aria-label="新增物料">+</button>
      </div>
    </header>

    <main class="main-viewport main-viewport--mobile" aria-label="主内容">
      <section class="mobile-summary">
        <article v-for="metric in metrics" :key="metric.label" class="metric-card">
          <span>{{ metric.label }}</span>
          <strong>{{ metric.value }}</strong>
        </article>
      </section>

      <section class="work-panel work-panel--mobile">
        <div class="panel-toolbar panel-toolbar--mobile">
          <div>
            <h1>库存列表</h1>
            <p>全部库位 / 全部分类</p>
          </div>
          <button class="primary-button primary-button--compact" type="button">新增</button>
        </div>

        <div class="mobile-list" aria-label="库存物料">
          <article v-for="item in stockItems" :key="item.sku" class="mobile-list-card">
            <div>
              <span class="mobile-list-card__code">{{ item.sku }}</span>
              <h2>{{ item.name }}</h2>
              <p>{{ item.location }}</p>
            </div>
            <div class="mobile-list-card__meta">
              <strong>{{ item.quantity }}</strong>
              <span class="status-pill" :class="`status-pill--${item.statusKind}`">
                {{ item.status }}
              </span>
            </div>
          </article>
        </div>
      </section>
    </main>

    <div v-if="navOpen" class="mobile-nav-layer" role="dialog" aria-modal="true" aria-label="导航面板">
      <button class="mobile-nav-backdrop" type="button" aria-label="关闭导航" @click="navOpen = false" />
      <aside class="mobile-nav-drawer">
        <div class="mobile-nav-drawer__header">
          <div>
            <p class="eyebrow">Navigation</p>
            <h2>工作区</h2>
          </div>
          <button class="icon-button" type="button" aria-label="关闭导航" @click="navOpen = false">×</button>
        </div>

        <div class="mobile-drawer-status">
          <span class="status-chip status-chip--ok">本地服务</span>
          <span class="status-chip">127.0.0.1:17890</span>
        </div>

        <div class="menu-section">
          <h3>模块导航</h3>
          <button
            v-for="item in navItems"
            :key="item.key"
            class="menu-item"
            :class="{ 'menu-item--active': item.key === activeNav }"
            type="button"
            @click="selectNav(item.key)"
          >
            {{ item.label }}
          </button>
        </div>

        <div class="menu-section">
          <h3>快捷操作</h3>
          <button class="menu-item menu-item--strong" type="button">批量移库</button>
          <button class="menu-item menu-item--strong" type="button">同步数据</button>
        </div>
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { metrics, navItems, stockItems } from '../mock/shellData'

const activeNav = ref('stock')
const navOpen = ref(false)

function selectNav(key: string) {
  activeNav.value = key
  navOpen.value = false
}
</script>
