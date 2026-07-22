<!--
  本文件拥有已登录应用区域的稳定响应式应用框架，属于 frontend 布局层。
  它通过 CSS 调整桌面和移动排布，始终只保留一个路由出口；不拥有平台 Shell 生命周期。
-->
<template>
  <div class="app-shell" data-shell="responsive">
    <header class="app-topbar">
      <div class="app-topbar__brand">
        <button
          ref="navTrigger"
          class="icon-button app-mobile-nav-trigger"
          type="button"
          aria-label="打开导航"
          aria-controls="app-navigation-pane"
          :aria-expanded="navOpen"
          @click="openNavigation"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M4 7h16M4 12h16M4 17h16" />
          </svg>
        </button>

        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>

        <div class="app-topbar__context">
          <span class="app-topbar__context-mark" aria-hidden="true">W</span>
          <span class="app-topbar__context-copy">
            <small>WineStock</small>
            <strong>{{ pageTitle }}</strong>
          </span>
        </div>
      </div>

      <div v-if="userDisplayName" class="app-account">
        <button
          class="app-account__trigger"
          type="button"
          :aria-expanded="accountMenuOpen"
          aria-controls="app-account-popover"
          :aria-label="`查看当前用户 ${userDisplayName} 的账户信息`"
          @click="toggleAccountMenu"
        >
          <AccountUserSummary :initials="userInitials" :display-name="userDisplayName" />
        </button>
        <button
          v-if="accountMenuOpen"
          class="account-popover-backdrop"
          type="button"
          aria-label="关闭账户信息"
          @click="closeAccountMenu"
        />
        <Transition name="account-popover">
          <AccountPopover
            v-if="accountMenuOpen"
            id="app-account-popover"
            :initials="userInitials"
            :display-name="userDisplayName"
            :show-user-summary="true"
            :logout-error="logoutError"
            :is-logging-out="isLoggingOut"
            @runtime-settings="openRuntimeSettings"
            @logout="handleLogout"
          />
        </Transition>
      </div>
    </header>

    <div class="app-workspace">
      <aside
        id="app-navigation-pane"
        class="app-navigation-pane"
        :class="{ 'app-navigation-pane--open': navOpen }"
        aria-label="导航面板"
      >
        <div class="app-navigation-pane__header">
          <div class="app-navigation-pane__brand">
            <span class="brand-mark" aria-hidden="true">W</span>
            <div>
              <strong>WineStock</strong>
              <span>应用导航</span>
            </div>
          </div>
          <button class="icon-button" type="button" aria-label="关闭导航" @click="closeNavigation">
            <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="m6 6 12 12M18 6 6 18" />
            </svg>
          </button>
        </div>

        <AppNavigationList :items="visibleNavigation" @navigate="closeNavigation" />
      </aside>

      <main class="app-content-pane" aria-label="主内容">
        <RouteContentView />
      </main>
    </div>

    <Transition name="mobile-nav-backdrop">
      <button
        v-if="navOpen"
        class="mobile-nav-backdrop"
        type="button"
        aria-label="关闭导航"
        @click="closeNavigation"
      />
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { authSession } from "../auth/session";
import AccountPopover from "../components/AccountPopover.vue";
import AccountUserSummary from "../components/AccountUserSummary.vue";
import AppNavigationList from "../components/AppNavigationList.vue";
import RouteContentView from "../components/RouteContentView.vue";
import { useAccountPopover } from "../composables/useAccountPopover";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { useShellLogout } from "../composables/useShellLogout";
import { NativeBackPriority } from "../navigation/nativeBack";
import { getVisibleAppNavigation } from "../router/navigation";

const DESKTOP_QUERY = "(min-width: 768px)";
const navOpen = ref(false);
const navTrigger = ref<HTMLButtonElement | null>(null);
const route = useRoute();
const router = useRouter();
const {
  accountMenuOpen,
  closeAccountMenu,
  toggleAccountMenu: toggleAccountPopover,
} = useAccountPopover();
const { handleLogout, isLoggingOut, logoutError } = useShellLogout();
const pageTitle = computed(() => route.meta.title);
const visibleNavigation = computed(() =>
  getVisibleAppNavigation(authSession.value?.user.permissions),
);
const userDisplayName = computed(() => authSession.value?.user.username ?? "");
const userInitials = computed(() =>
  Array.from(userDisplayName.value.trim()).slice(0, 2).join("").toUpperCase(),
);
let desktopMediaQuery: MediaQueryList | undefined;

useNativeBackHandler({
  id: "app-navigation-drawer",
  active: navOpen,
  priority: NativeBackPriority.Drawer,
  handle: () => {
    if (!navOpen.value) return { handled: false };
    closeNavigation();
    void nextTick(() => navTrigger.value?.focus());
    return { handled: true, reason: "drawer" };
  },
});

function openNavigation(): void {
  navTrigger.value?.blur();
  closeAccountMenu();
  navOpen.value = true;
}

function closeNavigation(): void {
  navOpen.value = false;
}

function toggleAccountMenu(): void {
  closeNavigation();
  logoutError.value = "";
  toggleAccountPopover();
}

function openRuntimeSettings(): void {
  closeAccountMenu();
  void router.push({
    name: "runtime-settings",
    query: { returnTo: route.fullPath },
  });
}

/** 断点变化只关闭临时 Drawer，不切换或重挂载应用框架和路由页面。 */
function handleDesktopQueryChange(event: MediaQueryListEvent): void {
  if (event.matches) closeNavigation();
}

/** Escape 只关闭移动导航；账户弹层由共享 composable 处理。 */
function handleEscape(event: KeyboardEvent): void {
  if (event.key === "Escape") closeNavigation();
}

watch(() => route.fullPath, closeNavigation);

onMounted(() => {
  document.addEventListener("keydown", handleEscape);
  desktopMediaQuery = window.matchMedia(DESKTOP_QUERY);
  desktopMediaQuery.addEventListener("change", handleDesktopQueryChange);
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", handleEscape);
  desktopMediaQuery?.removeEventListener("change", handleDesktopQueryChange);
});
</script>

<style lang="scss" src="./AppShell.scss"></style>
