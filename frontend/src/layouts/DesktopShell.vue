<!--
  本文件拥有桌面端应用壳，属于 frontend 布局层。
  它只组织顶部状态栏、左侧导航面板和右侧主内容，不拥有业务 API 或平台 shell 生命周期。
-->
<template>
  <div class="app-shell desktop-shell" data-shell="desktop">
    <header class="desktop-nav">
      <div class="brand-lockup">
        <span class="brand-mark">W</span>
        <span class="brand-name">WineStock</span>
      </div>

      <div class="desktop-nav__status" aria-label="服务状态">
        <span class="status-chip status-chip--ok">本地服务</span>
        <span class="status-chip">127.0.0.1:17890</span>
      </div>

      <div class="desktop-nav__actions">
        <button class="icon-button" type="button" aria-label="搜索">⌕</button>
        <button class="icon-button" type="button" aria-label="通知">!</button>
        <button class="avatar-button" type="button" aria-label="当前用户">AD</button>
      </div>
    </header>

    <div class="desktop-workspace">
      <main class="main-viewport main-viewport--desktop" aria-label="主内容">
        <aside class="desktop-navigation-pane" aria-label="导航面板">
          <div class="navigation-pane__header">
            <p class="eyebrow">Navigation</p>
            <h2>工作区</h2>
          </div>

          <div class="menu-section">
            <h3>模块导航</h3>
            <button
              v-for="item in navItems"
              :key="item.key"
              class="menu-item"
              :class="{ 'menu-item--active': item.key === activeNav }"
              type="button"
              @click="activeNav = item.key"
            >
              {{ item.label }}
            </button>
          </div>

          <div class="menu-section">
            <h3>库存分类</h3>
            <button v-for="filter in filters" :key="filter" class="menu-item" type="button">
              {{ filter }}
            </button>
          </div>

          <div class="menu-section">
            <h3>快捷操作</h3>
            <button class="menu-item menu-item--strong" type="button">批量移库</button>
            <button class="menu-item menu-item--strong" type="button">导出当前列表</button>
          </div>
        </aside>

        <section class="desktop-content-pane">
          <section class="content-header">
            <div>
              <p class="eyebrow">库存总览</p>
              <h1>{{ currentModule }}</h1>
            </div>
            <div class="content-actions">
              <button class="primary-button" type="button">新增物料</button>
            </div>
          </section>

          <section class="metric-grid" aria-label="关键指标">
            <article v-for="metric in metrics" :key="metric.label" class="metric-card">
              <span>{{ metric.label }}</span>
              <strong>{{ metric.value }}</strong>
              <small>{{ metric.caption }}</small>
            </article>
          </section>

          <section class="work-panel">
            <div class="panel-toolbar">
              <div>
                <h2>库存列表</h2>
                <p>当前筛选：全部库位 / 全部分类</p>
              </div>
              <div class="segmented-control" aria-label="视图切换">
                <button class="segmented-control__item segmented-control__item--active" type="button">
                  表格
                </button>
                <button class="segmented-control__item" type="button">批次</button>
              </div>
            </div>

            <div class="data-table" role="table" aria-label="库存物料">
              <div class="data-table__row data-table__row--head" role="row">
                <span role="columnheader">SKU</span>
                <span role="columnheader">名称</span>
                <span role="columnheader">库位</span>
                <span role="columnheader">库存</span>
                <span role="columnheader">状态</span>
              </div>
              <div v-for="item in stockItems" :key="item.sku" class="data-table__row" role="row">
                <span role="cell">{{ item.sku }}</span>
                <span role="cell">{{ item.name }}</span>
                <span role="cell">{{ item.location }}</span>
                <span role="cell">{{ item.quantity }}</span>
                <span role="cell">
                  <span class="status-pill" :class="`status-pill--${item.statusKind}`">
                    {{ item.status }}
                  </span>
                </span>
              </div>
            </div>
          </section>
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { filters, metrics, navItems, stockItems } from '../mock/shellData'

const activeNav = ref('stock')
const currentModule = computed(() => navItems.find((item) => item.key === activeNav.value)?.label ?? '库存')
</script>
