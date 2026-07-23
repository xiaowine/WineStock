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
        :class="{ 'menu-item--desktop-only': item.desktopOnly }"
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
import { computed } from "vue";
import type { AppNavigationIcon, AppNavigationItem } from "../router/navigation";

const props = defineProps<{
  /** 已按当前会话权限过滤的应用导航入口。 */
  items: readonly AppNavigationItem[];
}>();

const emit = defineEmits<{
  /** 用户选择入口后通知所属 Shell 收起临时导航。 */
  navigate: [];
}>();

const navigationGroups = computed(() =>
  [
    {
      id: "primary",
      label: "",
      items: props.items.filter((item) => item.group === "primary"),
    },
    {
      id: "management",
      label: "管理",
      items: props.items.filter((item) => item.group === "management"),
    },
  ].filter((group) => group.items.length > 0),
);

const navigationIconPaths: Record<AppNavigationIcon, string> = {
  dashboard: "M4 4h6v6H4z M14 4h6v4h-6z M14 12h6v8h-6z M4 14h6v6H4z",
  items: "m4 7.5 8-4.5 8 4.5v9L12 21l-8-4.5v-9Z M4 7.5l8 4.5 8-4.5 M12 12v9",
  "inbound-create": "M4 5h16v14H4z M8 9h5 M8 13h4 M12 2v6 M9 5l3 3 3-3",
  "inbound-orders": "M5 3h11l3 3v15H5z M16 3v4h4 M8 10h8 M12 9v7 M9 13l3 3 3-3 M8 20h8",
  "outbound-create": "M4 5h16v14H4z M8 9h8 M8 13h5 M12 22v-8 M9 17l3-3 3 3",
  "outbound-orders": "M5 3h11l3 3v15H5z M16 3v4h4 M8 10h8 M12 17v-7 M9 13l3-3 3 3 M8 20h8",
  "inbound-approvals": "M9 4H6v17h12V4h-3 M9 2h6v4H9z M8 13l2.5 2.5L16 10",
  "outbound-approvals": "M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6l7-3Z M8.5 12l2.2 2.2 4.8-5",
  locations:
    "M12 21s7-6.1 7-11a7 7 0 1 0-14 0c0 4.9 7 11 7 11Z M12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z",
  templates: "M4 6h11l3 3v12H4z M15 6v4h4 M7 13h8 M7 17h5 M7 3h11v4",
  substitutes: "M7 7h10 M7 17h10 M4 7l3-3 3 3 M20 17l-3 3-3-3",
  events: "M12 7v5l3 2 M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Z M4 5h2 M18 5h2",
  users:
    "M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z M3 21v-2a6 6 0 0 1 6-6h1 M16 11a4 4 0 0 0 0-8 M14 13h1a6 6 0 0 1 6 6v2",
};
</script>

<style lang="scss" src="./AppNavigationList.scss"></style>
