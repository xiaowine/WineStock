<!--
  本文件拥有桌面侧栏与移动 Drawer 共用的应用主导航列表，属于 frontend 通用导航组件。
  它只渲染已完成权限过滤的入口，不决定路由权限，也不拥有 Drawer 开关状态。
-->
<template>
  <nav class="app-navigation" aria-label="模块导航">
    <div
      v-for="group in navigationGroups"
      :key="group.id"
      class="navigation-group"
      :class="`navigation-group--${group.id}`"
    >
      <h3 v-if="group.label">{{ group.label }}</h3>
      <RouterLink
        v-for="item in group.items"
        :key="item.routeName"
        :to="{ name: item.routeName }"
        class="menu-item"
        active-class="menu-item--active"
        @click="emit('navigate')"
      >
        <svg class="menu-item__icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path :d="navigationIconPaths[item.icon]" />
        </svg>
        <span>{{ item.label }}</span>
      </RouterLink>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AppNavigationIcon, AppNavigationItem } from '../router/navigation'

const props = defineProps<{
  /** 已按当前会话权限过滤的应用导航入口。 */
  items: readonly AppNavigationItem[]
}>()

const emit = defineEmits<{
  /** 用户选择入口后通知所属 Shell 收起临时导航。 */
  navigate: []
}>()

const navigationGroups = computed(() =>
  [
    {
      id: 'primary',
      label: '',
      items: props.items.filter((item) => item.group === 'primary'),
    },
    {
      id: 'management',
      label: '管理',
      items: props.items.filter((item) => item.group === 'management'),
    },
  ].filter((group) => group.items.length > 0),
)

const navigationIconPaths: Record<AppNavigationIcon, string> = {
  dashboard: 'M4 4h6v6H4z M14 4h6v4h-6z M14 12h6v8h-6z M4 14h6v6H4z',
  items: 'm4 7.5 8-4.5 8 4.5v9L12 21l-8-4.5v-9Z M4 7.5l8 4.5 8-4.5 M12 12v9',
  users: 'M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z M3 21v-2a6 6 0 0 1 6-6h1 M16 11a4 4 0 0 0 0-8 M14 13h1a6 6 0 0 1 6 6v2',
}
</script>

<style lang="scss" src="./AppNavigationList.scss"></style>
