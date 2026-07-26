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

      <div v-if="userDisplayName || silentLocalMode" class="app-account">
        <button
          ref="accountTrigger"
          class="app-account__trigger"
          type="button"
          :aria-expanded="accountMenuOpen"
          aria-controls="app-account-popover"
          :aria-label="
            silentLocalMode ? '查看本机选项' : `查看当前用户 ${userDisplayName} 的账户与本机选项`
          "
          @click="toggleAccountMenu"
        >
          <AccountUserSummary :initials="accountInitials" :display-name="accountDisplayName" />
        </button>
        <button
          v-if="accountMenuOpen"
          class="account-popover-backdrop"
          type="button"
          aria-label="关闭账户与本机选项"
          @click="closeAccountMenu"
        />
        <Transition name="account-popover">
          <AccountPopover
            v-if="accountMenuOpen"
            id="app-account-popover"
            :initials="accountInitials"
            :display-name="accountDisplayName"
            :show-user-summary="!silentLocalMode"
            :show-lan-access="lanAccessUrls.length > 0"
            :show-logout="!silentLocalMode"
            :logout-error="logoutError"
            :is-logging-out="isLoggingOut"
            @runtime-settings="openRuntimeSettings"
            @lan-access="openLanAccessDialog"
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

        <AppNavigationList
          :items="visibleNavigation"
          :pending-route-name="pendingNavigationRouteName"
          @navigate="closeNavigation"
        />
      </aside>

      <main class="app-content-pane" aria-label="主内容">
        <RouteContentView />
        <!-- 触底安全区占位：必须是真实节点以撑开 scrollHeight，内容可仍延伸到导航栏下。 -->
        <div class="app-content-pane__end-inset" aria-hidden="true" />
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

    <LanAccessDialog
      :open="lanAccessDialogOpen"
      :urls="lanAccessUrls"
      @close="closeLanAccessDialog"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { authSession, localSilentAuthActive } from "../auth/session";
import AccountPopover from "../components/AccountPopover.vue";
import AccountUserSummary from "../components/AccountUserSummary.vue";
import AppNavigationList from "../components/AppNavigationList.vue";
import RouteContentView from "../components/RouteContentView.vue";
import LanAccessDialog from "../components/runtime/LanAccessDialog.vue";
import { useAccountPopover } from "../composables/useAccountPopover";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { useShellLogout } from "../composables/useShellLogout";
import { NativeBackPriority } from "../navigation/nativeBack";
import { schedulePrefetchAppPages } from "../router/appPageLoaders";
import { getVisibleAppNavigation } from "../router/navigation";
import { pendingNavigationRouteName } from "../router/navigationPending";
import { getUsableLanAccessUrls } from "../shell/lanAccess";
import { runtimeSnapshot } from "../shell/runtime";

const DESKTOP_QUERY = "(min-width: 768px)";
const navOpen = ref(false);
const navTrigger = ref<HTMLButtonElement | null>(null);
const accountTrigger = ref<HTMLButtonElement | null>(null);
const lanAccessDialogOpen = ref(false);
const route = useRoute();
const router = useRouter();
const {
  accountMenuOpen,
  closeAccountMenu,
  toggleAccountMenu: toggleAccountPopover,
} = useAccountPopover();
const { handleLogout, isLoggingOut, logoutError } = useShellLogout();
const pageTitle = computed(() => route.meta.title);
// 本机静默免登录模式：界面彻底无账号感，只保留中性的"本机选项"入口与运行设置能力。
const silentLocalMode = computed(() => localSilentAuthActive.value);
const visibleNavigation = computed(() => {
  const items = getVisibleAppNavigation(authSession.value?.user.permissions);
  return silentLocalMode.value ? items.filter((item) => item.routeName !== "users") : items;
});
const userDisplayName = computed(() => authSession.value?.user.username ?? "");
const accountDisplayName = computed(() => (silentLocalMode.value ? "本机" : userDisplayName.value));
const accountInitials = computed(() =>
  Array.from(accountDisplayName.value.trim()).slice(0, 2).join("").toUpperCase(),
);
const lanAccessUrls = computed(() => getUsableLanAccessUrls(runtimeSnapshot.value));
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

async function openLanAccessDialog(): Promise<void> {
  closeAccountMenu();
  await nextTick();
  accountTrigger.value?.focus();
  lanAccessDialogOpen.value = true;
}

function closeLanAccessDialog(): void {
  lanAccessDialogOpen.value = false;
}

/** 断点变化只关闭临时 Drawer，不切换或重挂载应用框架和路由页面。 */
function handleDesktopQueryChange(event: MediaQueryListEvent): void {
  if (event.matches) closeNavigation();
}

/** Escape 只关闭移动导航；账户弹层由共享 composable 处理。 */
function handleEscape(event: KeyboardEvent): void {
  if (event.key === "Escape") closeNavigation();
}

watch(lanAccessUrls, (urls) => {
  if (!urls.length) closeLanAccessDialog();
});

watch(
  () => route.fullPath,
  () => {
    closeNavigation();
    closeLanAccessDialog();
  },
);

onMounted(() => {
  document.addEventListener("keydown", handleEscape);
  desktopMediaQuery = window.matchMedia(DESKTOP_QUERY);
  desktopMediaQuery.addEventListener("change", handleDesktopQueryChange);
  // 空闲预取当前权限可见的页面 chunk，弱网下点击导航直接命中模块缓存。
  schedulePrefetchAppPages(visibleNavigation.value.map((item) => item.routeName));
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", handleEscape);
  desktopMediaQuery?.removeEventListener("change", handleDesktopQueryChange);
});
</script>

<style lang="scss" src="./AppShell.scss"></style>
